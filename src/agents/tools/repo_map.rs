//! Repository map tool for compact codebase orientation.

use anyhow::{Context, Result};
use ignore::WalkBuilder;
use regex::Regex;
use rig::tool::portable::PortableTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

use super::common::{current_repo_root, parameters_schema};

crate::define_tool_error!(RepoMapError);

const DEFAULT_TOKEN_BUDGET: u32 = 2_000;
const MAX_TOKEN_BUDGET: u32 = 8_000;
const MAX_FILE_BYTES: u64 = 400_000;
const DEFAULT_MAX_FILES: usize = 60;
const MAX_DEFINITIONS_PER_FILE: usize = 12;
const MAX_IMPORTS_PER_FILE: usize = 6;

static DEFINITION_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:unsafe\s+)?(?:fn|struct|enum|trait|type|mod)\s+([A-Za-z_][A-Za-z0-9_]*)",
        r"^\s*(?:export\s+)?(?:async\s+)?(?:function|class|interface|type|enum)\s+([A-Za-z_$][A-Za-z0-9_$]*)",
        r"^\s*(?:export\s+)?(?:const|let|var)\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*=",
        r"^\s*(?:async\s+)?(?:def|class)\s+([A-Za-z_][A-Za-z0-9_]*)",
        r"^\s*(?:func|type)\s+([A-Za-z_][A-Za-z0-9_]*)",
        r"^\s*(?:public\s+|private\s+|internal\s+|open\s+)?(?:fun|class|object|interface|struct|enum|protocol)\s+([A-Za-z_][A-Za-z0-9_]*)",
    ]
    .into_iter()
    .map(|pattern| Regex::new(pattern).expect("definition regex should compile"))
    .collect()
});

static IMPORT_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"^\s*(?:pub\s+)?use\s+(.+?);",
        r"^\s*mod\s+([A-Za-z_][A-Za-z0-9_]*);",
        r#"^\s*import\s+.+?\s+from\s+['"](.+?)['"]"#,
        r#"^\s*export\s+.+?\s+from\s+['"](.+?)['"]"#,
        r"^\s*from\s+([A-Za-z_][A-Za-z0-9_.]*)\s+import\s+",
        r"^\s*import\s+([A-Za-z_][A-Za-z0-9_.]*)",
        r#"^\s*import\s+["'](.+?)["']"#,
    ]
    .into_iter()
    .map(|pattern| Regex::new(pattern).expect("import regex should compile"))
    .collect()
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoMapTool;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct RepoMapArgs {
    #[serde(default = "default_token_budget")]
    pub token_budget: u32,
    #[serde(default)]
    pub mentioned_files: Vec<PathBuf>,
    #[serde(default = "default_max_files")]
    pub max_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoMap {
    pub files_analyzed: usize,
    pub files_shown: usize,
    pub changed_files: Vec<PathBuf>,
    pub mentioned_files: Vec<PathBuf>,
    pub content: String,
}

#[derive(Debug, Clone)]
struct FileSummary {
    path: PathBuf,
    score: usize,
    definitions: Vec<String>,
    imports: Vec<String>,
    changed: bool,
    mentioned: bool,
}

impl Default for RepoMapTool {
    fn default() -> Self {
        Self
    }
}

fn default_token_budget() -> u32 {
    DEFAULT_TOKEN_BUDGET
}

fn default_max_files() -> usize {
    DEFAULT_MAX_FILES
}

impl RepoMapTool {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    pub(super) fn build(repo_root: &Path, args: &RepoMapArgs) -> Result<RepoMap> {
        let changed_files = changed_files(repo_root);
        let mentioned_files = normalize_mentions(&args.mentioned_files);
        let mut summaries = collect_file_summaries(repo_root, &changed_files, &mentioned_files)?;
        let files_analyzed = summaries.len();
        summaries.sort_by_key(|summary| {
            (
                Reverse(summary.score),
                summary.path.components().count(),
                summary.path.clone(),
            )
        });

        let max_files = args.max_files.clamp(1, 200);
        summaries.truncate(max_files);
        let content = render_repo_map(
            &summaries,
            args.token_budget.clamp(50, MAX_TOKEN_BUDGET),
            files_analyzed,
        );

        Ok(RepoMap {
            files_analyzed,
            files_shown: summaries.len(),
            changed_files,
            mentioned_files,
            content,
        })
    }
}

impl PortableTool for RepoMapTool {
    const NAME: &'static str = "repo_map";
    type Error = RepoMapError;
    type Args = RepoMapArgs;
    type Output = RepoMap;

    fn description(&self) -> String {
        "Build a compact repository map with ranked source files, definitions, imports, and changed or mentioned-file signals. Use this before broad cross-file analysis when you need the codebase skeleton without reading every file.".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        parameters_schema::<RepoMapArgs>()
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "Defer synchronous tool work until polling inside the repository context"
    )]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let repo_root = current_repo_root().map_err(RepoMapError::from)?;
        Self::build(&repo_root, &args).map_err(RepoMapError::from)
    }
}

fn collect_file_summaries(
    repo_root: &Path,
    changed_files: &[PathBuf],
    mentioned_files: &[PathBuf],
) -> Result<Vec<FileSummary>> {
    let mut summaries = Vec::new();
    for entry in WalkBuilder::new(repo_root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .build()
        .filter_map(std::result::Result::ok)
    {
        let path = entry.path();
        if !entry
            .file_type()
            .is_some_and(|file_type| file_type.is_file())
            || !is_source_file(path)
            || is_large_file(path)
        {
            continue;
        }

        let relative_path = path
            .strip_prefix(repo_root)
            .context("walked path should be inside repo root")?
            .to_path_buf();
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let definitions = extract_matches(&content, &DEFINITION_PATTERNS, MAX_DEFINITIONS_PER_FILE);
        let imports = extract_matches(&content, &IMPORT_PATTERNS, MAX_IMPORTS_PER_FILE);
        let changed = changed_files
            .iter()
            .any(|changed| changed == &relative_path);
        let mentioned = mentioned_files
            .iter()
            .any(|mentioned| mentioned == &relative_path);
        let score = score_file(
            &relative_path,
            definitions.len(),
            imports.len(),
            changed,
            mentioned,
        );

        summaries.push(FileSummary {
            path: relative_path,
            score,
            definitions,
            imports,
            changed,
            mentioned,
        });
    }

    Ok(summaries)
}

fn changed_files(repo_root: &Path) -> Vec<PathBuf> {
    let output = Command::new("git")
        .args(["status", "--short"])
        .current_dir(repo_root)
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.get(3..))
        .filter_map(|path| path.split(" -> ").last())
        .map(PathBuf::from)
        .collect()
}

fn normalize_mentions(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths
        .iter()
        .map(|path| {
            PathBuf::from(
                path.to_string_lossy()
                    .replace('\\', "/")
                    .trim_start_matches("./")
                    .trim_start_matches('/'),
            )
        })
        .collect()
}

fn is_large_file(path: &Path) -> bool {
    path.metadata()
        .map_or(true, |metadata| metadata.len() > MAX_FILE_BYTES)
}

fn is_source_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension,
                "rs" | "ts"
                    | "tsx"
                    | "js"
                    | "jsx"
                    | "mjs"
                    | "cjs"
                    | "py"
                    | "go"
                    | "java"
                    | "kt"
                    | "kts"
                    | "swift"
                    | "rb"
                    | "lua"
                    | "sh"
                    | "zsh"
                    | "bash"
            )
        })
}

fn extract_matches(content: &str, patterns: &[Regex], limit: usize) -> Vec<String> {
    let mut matches = Vec::new();
    for line in content.lines() {
        for pattern in patterns {
            if let Some(captures) = pattern.captures(line)
                && let Some(item) = captures.get(1)
            {
                matches.push(item.as_str().trim().to_string());
                break;
            }
        }
        if matches.len() >= limit {
            break;
        }
    }
    matches
}

fn score_file(
    path: &Path,
    definitions_count: usize,
    imports_count: usize,
    changed: bool,
    mentioned: bool,
) -> usize {
    let path_text = path.to_string_lossy().to_ascii_lowercase();
    let mut score = definitions_count * 12 + imports_count * 3;

    if mentioned {
        score += 1_000;
    }
    if changed {
        score += 250;
    }
    if path.components().count() <= 2 {
        score += 40;
    }
    for keyword in [
        "main", "lib", "mod", "app", "config", "router", "agent", "tool", "auth", "api", "db",
        "state", "service",
    ] {
        if path_text.contains(keyword) {
            score += 20;
        }
    }

    score
}

fn render_repo_map(summaries: &[FileSummary], token_budget: u32, files_analyzed: usize) -> String {
    let mut output = format!(
        "Repository map: showing {} of {} analyzed source files.\n",
        summaries.len(),
        files_analyzed
    );

    for summary in summaries {
        let mut markers = Vec::new();
        if summary.changed {
            markers.push("changed");
        }
        if summary.mentioned {
            markers.push("mentioned");
        }
        let marker = if markers.is_empty() {
            String::new()
        } else {
            format!(" [{}]", markers.join(", "))
        };

        output.push_str(&format!(
            "\n{}{} (score {})",
            summary.path.display(),
            marker,
            summary.score
        ));
        if !summary.definitions.is_empty() {
            output.push_str(&format!("\n  defs: {}", summary.definitions.join(", ")));
        }
        if !summary.imports.is_empty() {
            output.push_str(&format!("\n  refs: {}", summary.imports.join(", ")));
        }
        output.push('\n');
    }

    let char_budget = usize::try_from(token_budget).map_or(usize::MAX / 4, |budget| budget * 4);
    truncate_chars(&output, char_budget)
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let mut truncated = text.chars().take(max_chars).collect::<String>();
    truncated.push_str("\n[repo_map truncated]");
    truncated
}
