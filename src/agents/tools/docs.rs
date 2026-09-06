//! Project documentation tool for Rig-based agents
//!
//! This tool fetches documentation files like README.md, CONTRIBUTING.md,
//! CHANGELOG.md, etc. from the project root.

use anyhow::Result;
use rig::tool::portable::PortableTool;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::path::{Path, PathBuf};

use super::common::{current_repo_root, parameters_schema};

// Use standard tool error macro for consistency
crate::define_tool_error!(DocsError);

const MAX_DOC_CHARS: usize = 20_000;
const MAX_CONTEXT_TOTAL_CHARS: usize = 8_000;
const MAX_CONTEXT_HEADINGS: usize = 6;
const MAX_CONTEXT_HIGHLIGHTS: usize = 3;
const CONTEXT_SUMMARY_CHAR_LIMIT: usize = 360;
const CONTEXT_HIGHLIGHT_CHAR_LIMIT: usize = 420;

const GENERIC_CONTEXT_KEYWORDS: &[&str] = &[
    "overview",
    "summary",
    "usage",
    "workflow",
    "development",
    "testing",
    "command",
    "config",
    "architecture",
    "convention",
    "release",
];

const README_CONTEXT_KEYWORDS: &[&str] = &[
    "feature",
    "getting started",
    "install",
    "quick start",
    "setup",
];

const AGENT_CONTEXT_KEYWORDS: &[&str] = &[
    "project",
    "provider",
    "tool",
    "instruction",
    "style",
    "git hygiene",
];

/// Tool for fetching project documentation files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDocs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContextDocKind {
    Readme,
    Agents,
}

#[derive(Debug, Clone)]
struct MarkdownSection {
    heading: String,
    body: String,
    position: usize,
}

/// Type of documentation to fetch
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum DocType {
    /// README file (README.md, README.rst, README.txt)
    #[default]
    Readme,
    /// Contributing guidelines (CONTRIBUTING.md)
    Contributing,
    /// Changelog (CHANGELOG.md, HISTORY.md)
    Changelog,
    /// License file (LICENSE, LICENSE.md)
    License,
    /// Code of conduct (`CODE_OF_CONDUCT.md`)
    CodeOfConduct,
    /// Agent/AI instructions (AGENTS.md, CLAUDE.md, .github/copilot-instructions.md)
    Agents,
    /// Project context: concise README + agent instructions summary
    Context,
    /// All documentation files
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ProjectDocsArgs {
    /// Type of documentation to fetch
    #[serde(default)]
    pub doc_type: DocType,
    /// Maximum characters to return (default: 20000, max: 20000).
    /// For `context`, this is a total shared budget across the returned snapshot.
    #[serde(default = "default_max_chars")]
    pub max_chars: usize,
}

fn default_max_chars() -> usize {
    MAX_DOC_CHARS
}

fn append_doc(
    output: &mut String,
    filename: &str,
    content: &str,
    max_chars: usize,
    truncated_hint: Option<&str>,
) {
    output.push_str(&format!("=== {} ===\n", filename));

    let char_count = content.chars().count();
    if char_count > max_chars {
        let truncated: String = content.chars().take(max_chars).collect();
        output.push_str(&truncated);

        if let Some(hint) = truncated_hint {
            output.push_str(&format!(
                "\n\n[... context snapshot truncated after {} chars; {} ...]\n",
                max_chars, hint
            ));
        } else {
            output.push_str(&format!(
                "\n\n[... truncated, {} more chars ...]\n",
                char_count - max_chars
            ));
        }
    } else {
        output.push_str(content);
    }

    output.push_str("\n\n");
}

fn readme_candidates() -> &'static [&'static str] {
    &[
        "README.md",
        "README.rst",
        "README.txt",
        "README",
        "readme.md",
    ]
}

fn agent_doc_candidates() -> &'static [&'static str] {
    &[
        "AGENTS.md",
        "CLAUDE.md",
        ".github/copilot-instructions.md",
        ".cursor/rules",
        "CODING_GUIDELINES.md",
    ]
}

fn find_first_existing_file(repo_root: &Path, candidates: &[&str]) -> Option<PathBuf> {
    candidates
        .iter()
        .map(|candidate| repo_root.join(candidate))
        .find(|path| path.exists())
}

fn is_markdown_heading(line: &str) -> bool {
    let hashes = line.chars().take_while(|&ch| ch == '#').count();
    hashes > 0 && hashes <= 6 && line.chars().nth(hashes) == Some(' ')
}

fn heading_title(heading: &str) -> &str {
    heading.trim_start_matches('#').trim()
}

fn is_list_item(line: &str) -> bool {
    line.starts_with("- ")
        || line.starts_with("* ")
        || line.starts_with("+ ")
        || line
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit() && line.contains(". "))
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return text.to_string();
    }

    let truncated: String = text.chars().take(max_chars).collect();
    format!("{truncated}...")
}

fn compact_excerpt(text: &str, max_chars: usize) -> String {
    let mut output = String::new();
    let mut in_code_block = false;
    let mut previous_was_blank = false;

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            continue;
        }

        if trimmed.is_empty() {
            if !output.is_empty() && !previous_was_blank {
                output.push_str("\n\n");
            }
            previous_was_blank = true;
            continue;
        }

        if output.chars().count() >= max_chars {
            break;
        }

        if is_list_item(trimmed) {
            if !output.is_empty() && !output.ends_with('\n') {
                output.push('\n');
            }
            output.push_str(trimmed);
            output.push('\n');
        } else {
            if !output.is_empty() && !output.ends_with('\n') && !output.ends_with(' ') {
                output.push(' ');
            }
            output.push_str(trimmed);
        }

        previous_was_blank = false;
    }

    truncate_chars(output.trim(), max_chars)
}

fn parse_markdown_sections(content: &str) -> (String, Vec<MarkdownSection>) {
    let mut intro_lines = Vec::new();
    let mut sections = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut current_lines = Vec::new();
    let mut in_code_block = false;

    for line in content.lines() {
        let trimmed = line.trim_end();
        let simplified = trimmed.trim();

        if simplified.starts_with("```") || simplified.starts_with("~~~") {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            continue;
        }

        if is_markdown_heading(simplified) {
            if let Some(heading) = current_heading.take() {
                sections.push(MarkdownSection {
                    heading,
                    body: current_lines.join("\n"),
                    position: sections.len(),
                });
                current_lines.clear();
            }

            current_heading = Some(simplified.to_string());
            continue;
        }

        if current_heading.is_some() {
            current_lines.push(trimmed.to_string());
        } else {
            intro_lines.push(trimmed.to_string());
        }
    }

    if let Some(heading) = current_heading {
        sections.push(MarkdownSection {
            heading,
            body: current_lines.join("\n"),
            position: sections.len(),
        });
    }

    (intro_lines.join("\n"), sections)
}

fn score_context_section(section: &MarkdownSection, kind: ContextDocKind) -> usize {
    let lower_heading = heading_title(&section.heading).to_ascii_lowercase();
    let mut score = 100usize.saturating_sub(section.position * 7);

    for keyword in GENERIC_CONTEXT_KEYWORDS {
        if lower_heading.contains(keyword) {
            score += 25;
        }
    }

    let extra_keywords = match kind {
        ContextDocKind::Readme => README_CONTEXT_KEYWORDS,
        ContextDocKind::Agents => AGENT_CONTEXT_KEYWORDS,
    };

    for keyword in extra_keywords {
        if lower_heading.contains(keyword) {
            score += 35;
        }
    }

    score
}

fn select_context_sections(
    sections: &[MarkdownSection],
    kind: ContextDocKind,
) -> Vec<&MarkdownSection> {
    let mut ranked = sections.iter().collect::<Vec<_>>();
    ranked.sort_by_key(|section| {
        (
            Reverse(score_context_section(section, kind)),
            section.position,
        )
    });
    ranked.truncate(MAX_CONTEXT_HIGHLIGHTS);
    ranked
}

fn render_context_doc(
    filename: &str,
    content: &str,
    kind: ContextDocKind,
    max_chars: usize,
) -> String {
    let (intro, sections) = parse_markdown_sections(content);
    let summary_source = if intro.trim().is_empty() {
        sections
            .first()
            .map_or(content, |section| section.body.as_str())
    } else {
        intro.as_str()
    };
    let summary = compact_excerpt(summary_source, CONTEXT_SUMMARY_CHAR_LIMIT);

    let headings = sections
        .iter()
        .take(MAX_CONTEXT_HEADINGS)
        .map(|section| heading_title(&section.heading).to_string())
        .collect::<Vec<_>>();

    let highlights = select_context_sections(&sections, kind)
        .into_iter()
        .filter_map(|section| {
            let snippet = compact_excerpt(&section.body, CONTEXT_HIGHLIGHT_CHAR_LIMIT);
            (!snippet.is_empty()).then(|| (heading_title(&section.heading).to_string(), snippet))
        })
        .collect::<Vec<_>>();

    let mut output = String::new();
    output.push_str(&format!("=== {filename} ===\n"));

    if !summary.is_empty() {
        output.push_str("Summary:\n");
        output.push_str(&summary);
        output.push_str("\n\n");
    }

    if !headings.is_empty() {
        output.push_str("Key sections: ");
        output.push_str(&headings.join(" | "));
        output.push_str("\n\n");
    }

    if !highlights.is_empty() {
        output.push_str("Highlights:\n");
        for (heading, snippet) in highlights {
            output.push_str(&format!("- {heading}: {snippet}\n"));
        }
    }

    truncate_chars(output.trim_end(), max_chars)
}

async fn build_context_output(repo_root: &Path, requested_max_chars: usize) -> Result<String> {
    let context_budget = requested_max_chars.min(MAX_CONTEXT_TOTAL_CHARS);
    let mut docs = Vec::new();

    if let Some(path) = find_first_existing_file(repo_root, readme_candidates()) {
        let content = read_repo_doc(repo_root, &path).await?;
        docs.push((ContextDocKind::Readme, path, content));
    }

    if let Some(path) = find_first_existing_file(repo_root, agent_doc_candidates()) {
        let content = read_repo_doc(repo_root, &path).await?;
        docs.push((ContextDocKind::Agents, path, content));
    }

    if docs.is_empty() {
        return Ok("No project context documentation found in project root.".to_string());
    }

    let mut output = String::from(
        "Concise project context. Use `project_docs(doc_type=\"readme\")` or \
`project_docs(doc_type=\"agents\")` for full targeted docs.\n\n",
    );

    let has_readme = docs
        .iter()
        .any(|(kind, _, _)| *kind == ContextDocKind::Readme);
    let has_agents = docs
        .iter()
        .any(|(kind, _, _)| *kind == ContextDocKind::Agents);

    let mut rendered = Vec::new();
    for (kind, path, content) in docs {
        let filename = path
            .strip_prefix(repo_root)
            .ok()
            .and_then(|relative| relative.to_str())
            .unwrap_or_else(|| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("doc")
            });

        let doc_budget = match (has_readme, has_agents, kind) {
            (true, true, ContextDocKind::Readme) => context_budget * 2 / 5,
            (true, true, ContextDocKind::Agents) => context_budget * 3 / 5,
            _ => context_budget,
        };

        rendered.push(render_context_doc(filename, &content, kind, doc_budget));
    }

    output.push_str(&rendered.join("\n\n"));
    Ok(truncate_chars(output.trim_end(), context_budget))
}

async fn read_repo_doc(repo_root: &Path, path: &Path) -> Result<String> {
    let canonical_root = tokio::fs::canonicalize(repo_root).await?;
    let canonical_path = tokio::fs::canonicalize(path).await?;
    anyhow::ensure!(
        canonical_path.starts_with(&canonical_root),
        "Documentation path escapes repository boundaries"
    );
    Ok(tokio::fs::read_to_string(canonical_path).await?)
}

impl PortableTool for ProjectDocs {
    const NAME: &'static str = "project_docs";
    type Error = DocsError;
    type Args = ProjectDocsArgs;
    type Output = String;

    fn description(&self) -> String {
        "Fetch project documentation for context. Types: readme, contributing, changelog, license, codeofconduct, agents (AGENTS.md/CLAUDE.md), context (compact README + agent-instructions snapshot), all"
                    .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        parameters_schema::<ProjectDocsArgs>()
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let current_dir = current_repo_root().map_err(DocsError::from)?;
        let max_chars = args.max_chars.min(MAX_DOC_CHARS);

        if matches!(args.doc_type, DocType::Context) {
            return build_context_output(&current_dir, max_chars)
                .await
                .map_err(DocsError::from);
        }

        let files_to_check = match args.doc_type {
            DocType::Readme => readme_candidates().to_vec(),
            DocType::Contributing => vec!["CONTRIBUTING.md", "CONTRIBUTING", "contributing.md"],
            DocType::Changelog => vec![
                "CHANGELOG.md",
                "CHANGELOG",
                "HISTORY.md",
                "CHANGES.md",
                "changelog.md",
            ],
            DocType::License => vec!["LICENSE", "LICENSE.md", "LICENSE.txt", "license"],
            DocType::CodeOfConduct => vec!["CODE_OF_CONDUCT.md", "code_of_conduct.md"],
            DocType::Agents => agent_doc_candidates().to_vec(),
            DocType::Context => Vec::new(),
            DocType::All => vec![
                "README.md",
                "AGENTS.md",
                "CLAUDE.md",
                "CONTRIBUTING.md",
                "CHANGELOG.md",
                "CODE_OF_CONDUCT.md",
            ],
        };

        let mut output = String::new();
        let mut found_any = false;
        // Track if we found an agent instructions file (AGENTS.md often symlinks to CLAUDE.md)
        let mut found_agent_doc = false;

        for filename in files_to_check {
            // Skip CLAUDE.md if we already found AGENTS.md (avoid duplicate from symlink)
            if filename == "CLAUDE.md" && found_agent_doc {
                continue;
            }

            let path: PathBuf = current_dir.join(filename);
            if path.exists() {
                match read_repo_doc(&current_dir, &path).await {
                    Ok(content) => {
                        found_any = true;

                        // Mark that we found an agent doc file
                        if filename == "AGENTS.md" {
                            found_agent_doc = true;
                        }

                        append_doc(&mut output, filename, &content, max_chars, None);

                        // For single doc types, return after finding first match
                        // Context and All gather multiple files
                        if !matches!(args.doc_type, DocType::All) {
                            break;
                        }
                    }
                    Err(e) => {
                        output.push_str(&format!("Error reading {}: {}\n", filename, e));
                    }
                }
            }
        }

        if !found_any {
            output = format!(
                "No {:?} documentation found in project root.",
                args.doc_type
            );
        }

        Ok(output)
    }
}
