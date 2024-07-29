use super::FileAnalyzer;
use crate::git::FileChange;
use regex::Regex;

/// Analyzer for Markdown files
pub struct MarkdownAnalyzer;

impl FileAnalyzer for MarkdownAnalyzer {
    fn analyze(&self, _file: &str, change: &FileChange) -> Vec<String> {
        let mut analysis = Vec::new();

        // Check for new or modified headers
        if let Some(headers) = extract_modified_headers(&change.diff) {
            analysis.push(format!("Modified headers: {}", headers.join(", ")));
        }

        // Check for changes in lists
        if has_list_changes(&change.diff) {
            analysis.push("List structures have been modified".to_string());
        }

        // Check for changes in code blocks
        if has_code_block_changes(&change.diff) {
            analysis.push("Code blocks have been modified".to_string());
        }

        // Check for changes in links
        if has_link_changes(&change.diff) {
            analysis.push("Links have been modified".to_string());
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "Markdown file"
    }
}

fn extract_modified_headers(diff: &str) -> Option<Vec<String>> {
    let re = Regex::new(r"[+-]\s*(#{1,6})\s+(.+)").unwrap();
    let headers: Vec<String> = re
        .captures_iter(diff)
        .filter_map(|cap| cap.get(2).map(|m| m.as_str().to_string()))
        .collect();

    if headers.is_empty() {
        None
    } else {
        Some(headers)
    }
}

fn has_list_changes(diff: &str) -> bool {
    let re = Regex::new(r"[+-]\s*[-*+]\s+").unwrap();
    re.is_match(diff)
}

fn has_code_block_changes(diff: &str) -> bool {
    let re = Regex::new(r"[+-]\s*```").unwrap();
    re.is_match(diff)
}

fn has_link_changes(diff: &str) -> bool {
    let re = Regex::new(r"[+-]\s*\[.+\]\(.+\)").unwrap();
    re.is_match(diff)
}
