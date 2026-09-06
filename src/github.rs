use crate::git::GitRepo;
use crate::types::{Finding, Review as CodeReview};
use anyhow::{Context, Result, anyhow, bail};
use octocrab::models::pulls::{PullRequest, Review as GitHubReview, ReviewAction};
use octocrab::{Octocrab, params};
use regex::Regex;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use url::Url;

mod review_target;
pub use review_target::ReviewTarget;

static BACKTICK_LOCATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"`([^`\s]+):(\d+)`").expect("backtick location regex should compile")
});
static PLAIN_LOCATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([A-Za-z0-9_./-]+\.[A-Za-z0-9_-]+):(\d+)")
        .expect("plain location regex should compile")
});
static HUNK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@")
        .expect("unified diff hunk regex should compile")
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubRepository {
    pub owner: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestTemplate {
    pub path: String,
    pub body: String,
}

#[derive(Debug, Clone, Copy)]
pub struct ReviewPublishOptions {
    pub event: ReviewAction,
    pub inline_comments: bool,
}

pub struct GitHubClient {
    crab: Octocrab,
    repo: GitHubRepository,
}

impl GitHubClient {
    pub fn from_git_repo(repo: &GitRepo) -> Result<Self> {
        let remote_url = github_remote_url(repo)?;
        let github_repo = GitHubRepository::parse(&remote_url)?;
        let token =
            gh_token::get().map_err(|e| anyhow!("GitHub authentication unavailable: {e}"))?;
        let crab = Octocrab::builder()
            .personal_token(token)
            .build()
            .context("Failed to initialize GitHub client")?;

        Ok(Self {
            crab,
            repo: github_repo,
        })
    }

    pub async fn resolve_pull_number(
        &self,
        explicit_pull_number: Option<u64>,
        git_repo: &GitRepo,
    ) -> Result<u64> {
        if let Some(number) = explicit_pull_number {
            return Ok(number);
        }

        let branch = git_repo
            .get_current_branch()
            .context("Could not infer PR: failed to read current branch")?;
        if branch == "HEAD detached" {
            bail!("Could not infer PR from a detached HEAD; pass --pr <number>");
        }

        self.find_open_pull_for_branch(&branch).await
    }

    pub async fn update_pull_body(&self, pull_number: u64, body: &str) -> Result<PullRequest> {
        self.crab
            .pulls(&self.repo.owner, &self.repo.name)
            .update(pull_number)
            .body(body)
            .send()
            .await
            .with_context(|| format!("Failed to update PR #{pull_number}"))
    }

    pub async fn pull_body(&self, pull_number: u64) -> Result<String> {
        let pull = self
            .crab
            .pulls(&self.repo.owner, &self.repo.name)
            .get(pull_number)
            .await
            .with_context(|| format!("Failed to fetch PR #{pull_number}"))?;

        Ok(pull.body.unwrap_or_default())
    }

    pub async fn publish_review(
        &self,
        pull_number: u64,
        body: &str,
        options: ReviewPublishOptions,
        target: &ReviewTarget,
    ) -> Result<GitHubReview> {
        self.publish_review_with_comments(
            pull_number,
            ReviewSource::Markdown(body),
            options,
            target,
        )
        .await
    }

    pub async fn publish_structured_review(
        &self,
        pull_number: u64,
        review: &CodeReview,
        options: ReviewPublishOptions,
        target: &ReviewTarget,
    ) -> Result<GitHubReview> {
        self.publish_review_with_comments(
            pull_number,
            ReviewSource::Structured(review),
            options,
            target,
        )
        .await
    }

    async fn publish_review_with_comments(
        &self,
        pull_number: u64,
        review: ReviewSource<'_>,
        options: ReviewPublishOptions,
        target: &ReviewTarget,
    ) -> Result<GitHubReview> {
        let pull = self
            .crab
            .pulls(&self.repo.owner, &self.repo.name)
            .get(pull_number)
            .await
            .with_context(|| format!("Failed to fetch PR #{pull_number}"))?;
        target.validate(&pull.base.ref_field, &pull.head.sha)?;
        let review_body = review.body(&self.repo, &target.head_sha);
        let comments = if options.inline_comments {
            self.validated_inline_comments(pull_number, review).await?
        } else {
            Vec::new()
        };

        // The diff endpoint is mutable; recheck after retrieving inline locations.
        let current = self.review_target(pull_number).await?;
        target.validate(&current.base_ref, &current.head_sha)?;

        let route = format!(
            "/repos/{owner}/{repo}/pulls/{pull_number}/reviews",
            owner = self.repo.owner,
            repo = self.repo.name,
        );
        let payload = serde_json::json!({
            "body": review_body,
            "event": options.event,
            "commit_id": target.head_sha,
            "comments": comments,
        });

        self.crab
            .post(route, Some(&payload))
            .await
            .with_context(|| format!("Failed to publish review on PR #{pull_number}"))
    }

    pub fn repo(&self) -> &GitHubRepository {
        &self.repo
    }

    /// Capture the immutable commits before generating a pull-request review.
    pub async fn review_target(&self, pull_number: u64) -> Result<ReviewTarget> {
        let pull = self
            .crab
            .pulls(&self.repo.owner, &self.repo.name)
            .get(pull_number)
            .await
            .with_context(|| format!("Failed to fetch PR #{pull_number}"))?;
        Ok(ReviewTarget {
            base_ref: pull.base.ref_field,
            base_sha: pull.base.sha,
            head_sha: pull.head.sha,
        })
    }

    async fn find_open_pull_for_branch(&self, branch: &str) -> Result<u64> {
        let same_repo_head = format!("{}:{branch}", self.repo.owner);
        let page = self
            .crab
            .pulls(&self.repo.owner, &self.repo.name)
            .list()
            .state(params::State::Open)
            .head(same_repo_head)
            .per_page(10)
            .send()
            .await
            .with_context(|| format!("Failed to search open PRs for branch `{branch}`"))?;

        if let Some(number) = single_pull_number(&page.items) {
            return Ok(number);
        }

        let page = self
            .crab
            .pulls(&self.repo.owner, &self.repo.name)
            .list()
            .state(params::State::Open)
            .per_page(100)
            .send()
            .await
            .context("Failed to list open PRs")?;
        let matches: Vec<&PullRequest> = page
            .items
            .iter()
            .filter(|pull| pull.head.ref_field == branch)
            .collect();

        match matches.as_slice() {
            [pull] => Ok(pull.number),
            [] => bail!("No open GitHub PR found for branch `{branch}`; pass --pr <number>"),
            _ => bail!("Multiple open GitHub PRs found for branch `{branch}`; pass --pr <number>"),
        }
    }

    async fn validated_inline_comments(
        &self,
        pull_number: u64,
        review: ReviewSource<'_>,
    ) -> Result<Vec<Value>> {
        let diff = self
            .crab
            .pulls(&self.repo.owner, &self.repo.name)
            .get_diff(pull_number)
            .await
            .with_context(|| format!("Failed to fetch PR #{pull_number} diff"))?;
        let reviewable_lines = parse_reviewable_lines(&diff);
        let candidates = review.inline_comment_candidates();

        Ok(candidates
            .into_iter()
            .filter(|candidate| candidate.is_reviewable(&reviewable_lines))
            .map(|candidate| {
                let mut comment = serde_json::json!({
                    "path": candidate.path,
                    "line": candidate.line,
                    "side": "RIGHT",
                    "body": candidate.body,
                });
                if let Some(start_line) = candidate.start_line
                    && let Some(object) = comment.as_object_mut()
                {
                    object.insert("start_line".to_string(), serde_json::json!(start_line));
                    object.insert("start_side".to_string(), serde_json::json!("RIGHT"));
                }
                comment
            })
            .collect())
    }
}

#[derive(Debug, Clone, Copy)]
enum ReviewSource<'a> {
    Markdown(&'a str),
    Structured(&'a CodeReview),
}

impl ReviewSource<'_> {
    fn body(self, repo: &GitHubRepository, sha: &str) -> String {
        match self {
            Self::Markdown(body) => body.to_string(),
            Self::Structured(review) => review_body_with_permalinks(repo, review, sha),
        }
    }

    fn inline_comment_candidates(self) -> Vec<InlineCommentCandidate> {
        match self {
            Self::Markdown(body) => extract_inline_comment_candidates(body),
            Self::Structured(review) => extract_structured_inline_comment_candidates(review),
        }
    }
}

pub fn find_pull_request_template(repo_root: &Path) -> Result<Option<PullRequestTemplate>> {
    for path in singular_template_paths(repo_root) {
        if path.is_file() {
            return read_template(repo_root, &path).map(Some);
        }
    }

    for dir in template_directories(repo_root) {
        if let Some(template) = directory_template(repo_root, &dir)? {
            return Ok(Some(template));
        }
    }

    Ok(None)
}

fn singular_template_paths(repo_root: &Path) -> [PathBuf; 3] {
    [
        repo_root.join(".github/pull_request_template.md"),
        repo_root.join("pull_request_template.md"),
        repo_root.join("docs/pull_request_template.md"),
    ]
}

fn template_directories(repo_root: &Path) -> [PathBuf; 3] {
    [
        repo_root.join(".github/PULL_REQUEST_TEMPLATE"),
        repo_root.join("PULL_REQUEST_TEMPLATE"),
        repo_root.join("docs/PULL_REQUEST_TEMPLATE"),
    ]
}

fn directory_template(repo_root: &Path, dir: &Path) -> Result<Option<PullRequestTemplate>> {
    if !dir.is_dir() {
        return Ok(None);
    }

    let default_path = dir.join("pull_request_template.md");
    if default_path.is_file() {
        return read_template(repo_root, &default_path).map(Some);
    }

    let markdown_templates = markdown_files(dir)?;
    if markdown_templates.len() == 1 {
        read_template(repo_root, &markdown_templates[0]).map(Some)
    } else {
        Ok(None)
    }
}

fn markdown_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("Failed to read {}", dir.display()))? {
        let path = entry?.path();
        if path.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn read_template(repo_root: &Path, path: &Path) -> Result<PullRequestTemplate> {
    let body = fs::read_to_string(path)
        .with_context(|| format!("Failed to read PR template {}", path.display()))?;
    let relative_path = path
        .strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();

    Ok(PullRequestTemplate {
        path: relative_path,
        body,
    })
}

impl GitHubRepository {
    pub fn parse(remote_url: &str) -> Result<Self> {
        if let Some(path) = remote_url.strip_prefix("git@github.com:") {
            return Self::parse_path(path);
        }

        let url = Url::parse(remote_url)
            .with_context(|| format!("Could not parse GitHub remote URL `{remote_url}`"))?;
        if url.host_str() != Some("github.com") {
            bail!("Only github.com remotes are supported for GitHub publishing");
        }
        Self::parse_path(url.path().trim_start_matches('/'))
    }

    fn parse_path(path: &str) -> Result<Self> {
        let clean_path = path.trim_end_matches(".git").trim_end_matches('/');
        let mut parts = clean_path.split('/');
        let owner = parts
            .next()
            .filter(|part| !part.is_empty())
            .ok_or_else(|| anyhow!("GitHub remote URL is missing an owner"))?;
        let name = parts
            .next()
            .filter(|part| !part.is_empty())
            .ok_or_else(|| anyhow!("GitHub remote URL is missing a repository name"))?;

        if parts.next().is_some() {
            bail!("GitHub remote URL has an unexpected path shape");
        }

        Ok(Self {
            owner: owner.to_string(),
            name: name.to_string(),
        })
    }
}

fn github_remote_url(repo: &GitRepo) -> Result<String> {
    if let Some(url) = repo.get_remote_url() {
        return Ok(url.to_string());
    }

    let raw_repo = repo.open_repo()?;
    let remote = raw_repo
        .find_remote("origin")
        .or_else(|_| {
            let remotes = raw_repo.remotes()?;
            let remote_name = remotes
                .iter()
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .next()
                .ok_or(git2::Error::from_str("No git remotes configured"))?;
            raw_repo.find_remote(remote_name)
        })
        .context("Could not find a git remote for GitHub publishing")?;

    remote
        .url()
        .map(std::string::ToString::to_string)
        .context("Git remote has no valid URL")
}

fn single_pull_number(pulls: &[PullRequest]) -> Option<u64> {
    match pulls {
        [pull] => Some(pull.number),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InlineCommentCandidate {
    path: String,
    start_line: Option<u64>,
    line: u64,
    body: String,
}

impl InlineCommentCandidate {
    fn is_reviewable(&self, reviewable_lines: &HashMap<String, HashSet<u64>>) -> bool {
        reviewable_lines.get(&self.path).is_some_and(|lines| {
            let start = self.start_line.unwrap_or(self.line).min(self.line);
            let end = self.start_line.unwrap_or(self.line).max(self.line);
            (start..=end).all(|line| lines.contains(&line))
        })
    }
}

fn extract_inline_comment_candidates(review: &str) -> Vec<InlineCommentCandidate> {
    let lines: Vec<&str> = review.lines().collect();
    let mut candidates = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        if let Some((path, line_number)) = extract_location(line)
            && looks_like_finding(line)
        {
            let body = finding_body(&lines, index);
            candidates.push(InlineCommentCandidate {
                path,
                start_line: None,
                line: line_number,
                body,
            });
        }
        index += 1;
    }

    candidates
}

fn extract_structured_inline_comment_candidates(
    review: &CodeReview,
) -> Vec<InlineCommentCandidate> {
    review
        .visible_findings()
        .into_iter()
        .map(inline_comment_candidate_from_finding)
        .collect()
}

fn inline_comment_candidate_from_finding(finding: &Finding) -> InlineCommentCandidate {
    let start = finding.start_line.min(finding.end_line);
    let end = finding.start_line.max(finding.end_line);
    let start_line = (start != end).then_some(u64::from(start));

    InlineCommentCandidate {
        path: normalize_github_path(&finding.file),
        start_line,
        line: u64::from(end),
        body: finding.raw_inline_body(),
    }
}

fn review_body_with_permalinks(repo: &GitHubRepository, review: &CodeReview, sha: &str) -> String {
    let mut body = review.raw_content();
    let findings = review.visible_findings();
    if findings.is_empty() {
        return body;
    }

    body.push_str("\n## GitHub Permalinks\n");
    for finding in findings {
        body.push_str(&format!(
            "\n- {}: {}\n",
            finding.id.0,
            permalink_for_finding(repo, finding, sha)
        ));
    }

    body
}

fn permalink_for_finding(repo: &GitHubRepository, finding: &Finding, sha: &str) -> String {
    let path = percent_encode_github_path(&normalize_github_path(&finding.file));
    let start = finding.start_line.min(finding.end_line);
    let end = finding.start_line.max(finding.end_line);
    let line = if start == end {
        format!("L{start}")
    } else {
        format!("L{start}-L{end}")
    };

    format!(
        "https://github.com/{}/{}/blob/{}/{}#{}",
        repo.owner, repo.name, sha, path, line
    )
}

fn normalize_github_path(path: &Path) -> String {
    normalize_github_path_str(&path.to_string_lossy())
}

fn normalize_github_path_str(path: &str) -> String {
    path.replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string()
}

fn percent_encode_github_path(path: &str) -> String {
    path.split('/')
        .map(percent_encode_path_segment)
        .collect::<Vec<_>>()
        .join("/")
}

fn percent_encode_path_segment(segment: &str) -> String {
    let mut encoded = String::new();
    for byte in segment.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn extract_location(line: &str) -> Option<(String, u64)> {
    BACKTICK_LOCATION_RE
        .captures(line)
        .or_else(|| PLAIN_LOCATION_RE.captures(line))
        .and_then(|captures| {
            let path = normalize_github_path_str(captures.get(1)?.as_str());
            let line = captures.get(2)?.as_str().parse().ok()?;
            Some((path, line))
        })
}

fn looks_like_finding(line: &str) -> bool {
    ["[CRITICAL]", "[HIGH]", "[MEDIUM]", "[LOW]"]
        .iter()
        .any(|severity| line.contains(severity))
}

fn finding_body(lines: &[&str], start: usize) -> String {
    let mut body = Vec::new();
    let mut index = start;

    while index < lines.len() {
        let line = lines[index];
        if index > start && starts_new_finding_or_section(line) {
            break;
        }
        body.push(line.trim());
        index += 1;
    }

    body.join("\n").trim().to_string()
}

fn starts_new_finding_or_section(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("# ") || trimmed.starts_with("## ") || looks_like_finding(trimmed)
}

fn parse_reviewable_lines(diff: &str) -> HashMap<String, HashSet<u64>> {
    let mut lines_by_path = HashMap::new();
    let mut current_path: Option<String> = None;
    let mut new_line: Option<u64> = None;

    for line in diff.lines() {
        if let Some(path) = line.strip_prefix("+++ b/") {
            current_path = Some(path.to_string());
            continue;
        }

        if let Some(captures) = HUNK_RE.captures(line) {
            new_line = captures.get(1).and_then(|m| m.as_str().parse().ok());
            continue;
        }

        let Some(path) = current_path.as_ref() else {
            continue;
        };
        let Some(line_number) = new_line else {
            continue;
        };

        if let Some(b'+' | b' ') = line.as_bytes().first().copied() {
            lines_by_path
                .entry(path.clone())
                .or_insert_with(HashSet::new)
                .insert(line_number);
            new_line = Some(line_number + 1);
        }
    }

    lines_by_path
}

#[cfg(test)]
mod tests;
