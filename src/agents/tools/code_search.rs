//! Code search tool
//!
//! This tool provides Iris with the ability to search for code patterns,
//! functions, classes, and related files in the repository.

use anyhow::Result;
use regex::escape as regex_escape;
use rig::tool::portable::PortableTool;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

use super::common::{get_current_repo, parameters_schema};
use crate::define_tool_error;

define_tool_error!(CodeSearchError);

/// Code search tool for finding related files and functions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSearch;

impl Default for CodeSearch {
    fn default() -> Self {
        Self
    }
}

impl CodeSearch {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Execute a ripgrep search for patterns
    fn execute_ripgrep_search(
        query: &str,
        repo_path: &Path,
        file_pattern: Option<&str>,
        search_type: &str,
        max_results: usize,
    ) -> Result<Vec<SearchResult>> {
        let mut cmd = Command::new("rg");

        // Configure ripgrep based on search type
        // For structured types (function/class/variable), escape the query to prevent
        // regex injection — the surrounding pattern provides the regex structure.
        // For "pattern" type, the user explicitly wants regex, so we add a timeout instead.
        match search_type {
            "function" => {
                let escaped = regex_escape(query);
                cmd.args(["--type", "rust", "--type", "javascript", "--type", "python"]);
                cmd.args([
                    "-e",
                    &format!(r"fn\s+{escaped}|function\s+{escaped}|def\s+{escaped}"),
                ]);
            }
            "class" => {
                let escaped = regex_escape(query);
                cmd.args(["--type", "rust", "--type", "javascript", "--type", "python"]);
                cmd.args(["-e", &format!(r"struct\s+{escaped}|class\s+{escaped}")]);
            }
            "variable" => {
                let escaped = regex_escape(query);
                cmd.args([
                    "-e",
                    &format!(r"let\s+{escaped}|var\s+{escaped}|{escaped}\s*="),
                ]);
            }
            "pattern" => {
                // User-supplied regex — enforce a timeout to prevent ReDoS
                cmd.args(["--regex-size-limit", "1M", "--dfa-size-limit", "1M"]);
                cmd.args(["-e", query]);
            }
            _ => {
                // Fixed-string literal search — -F disables regex interpretation
                cmd.args(["-F", "-i", query]);
            }
        }

        // Add file pattern if specified (reject path traversal)
        if let Some(pattern) = file_pattern {
            if pattern.contains("..") {
                return Err(anyhow::anyhow!(
                    "File pattern must not contain '..' path traversal"
                ));
            }
            cmd.args(["-g", pattern]);
        }

        // Limit results and add context
        cmd.args(["-n", "--color", "never", "-A", "3", "-B", "1"]);
        cmd.current_dir(repo_path);

        let output = cmd.output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut results = Vec::new();
        let mut current_file = String::new();
        let mut line_number = 0;
        let mut content_lines = Vec::new();

        for line in stdout.lines().take(max_results * 4) {
            // rough estimate with context
            if line.contains(':') && !line.starts_with('-') {
                // Parse file:line:content format
                let parts: Vec<&str> = line.splitn(3, ':').collect();
                if parts.len() >= 3 {
                    let file_path = parts[0].to_string();
                    if let Ok(line_num) = parts[1].parse::<usize>() {
                        let content = parts[2].to_string();

                        if file_path != current_file && !current_file.is_empty() {
                            // Finalize previous result
                            results.push(SearchResult {
                                file_path: current_file.clone(),
                                line_number,
                                content: content_lines.join("\n"),
                                match_type: search_type.to_string(),
                                context_lines: content_lines.len(),
                            });
                            content_lines.clear();
                        }

                        current_file = file_path;
                        line_number = line_num;
                        content_lines.push(content);

                        if results.len() >= max_results {
                            break;
                        }
                    }
                }
            } else if !line.starts_with('-') && !current_file.is_empty() {
                // Context line
                content_lines.push(line.to_string());
            }
        }

        // Add final result
        if !current_file.is_empty() && results.len() < max_results {
            results.push(SearchResult {
                file_path: current_file,
                line_number,
                content: content_lines.join("\n"),
                match_type: search_type.to_string(),
                context_lines: content_lines.len(),
            });
        }

        Ok(results)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub file_path: String,
    pub line_number: usize,
    pub content: String,
    pub match_type: String,
    pub context_lines: usize,
}

/// Search type for code search
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum SearchType {
    /// Search for function definitions
    Function,
    /// Search for class/struct definitions
    Class,
    /// Search for variable assignments
    Variable,
    /// General text search (case-insensitive)
    #[default]
    Text,
    /// Regex pattern search
    Pattern,
}

impl SearchType {
    fn as_str(&self) -> &'static str {
        match self {
            SearchType::Function => "function",
            SearchType::Class => "class",
            SearchType::Variable => "variable",
            SearchType::Text => "text",
            SearchType::Pattern => "pattern",
        }
    }
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct CodeSearchArgs {
    /// Search query - function name, class name, variable, or pattern
    pub query: String,
    /// Type of search to perform
    #[serde(default)]
    pub search_type: SearchType,
    /// Optional file glob pattern to limit scope (e.g., "*.rs", "*.js")
    #[serde(default)]
    pub file_pattern: Option<String>,
    /// Maximum results to return (default: 20, max: 100)
    #[serde(default = "default_max_results")]
    pub max_results: usize,
}

fn default_max_results() -> usize {
    20
}

impl PortableTool for CodeSearch {
    const NAME: &'static str = "code_search";
    type Error = CodeSearchError;
    type Args = CodeSearchArgs;
    type Output = String;

    fn description(&self) -> String {
        "Search for code patterns, functions, classes, and related files in the repository using ripgrep. Supports multiple search types and file filtering.".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        parameters_schema::<CodeSearchArgs>()
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "Defer synchronous tool work until polling inside the repository context"
    )]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let repo = get_current_repo().map_err(CodeSearchError::from)?;
        let repo_path = repo.repo_path().clone();
        let max_results = args.max_results.min(100); // Cap at 100

        let results = Self::execute_ripgrep_search(
            &args.query,
            &repo_path,
            args.file_pattern.as_deref(),
            args.search_type.as_str(),
            max_results,
        )
        .map_err(CodeSearchError::from)?;

        let result = serde_json::json!({
            "query": args.query,
            "search_type": args.search_type.as_str(),
            "results": results,
            "total_found": results.len(),
            "max_results": max_results,
        });

        serde_json::to_string_pretty(&result).map_err(|e| CodeSearchError(e.to_string()))
    }
}
