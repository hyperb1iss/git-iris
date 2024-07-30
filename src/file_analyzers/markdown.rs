use super::FileAnalyzer;
use crate::context::StagedFile;
use regex::Regex;

pub struct MarkdownAnalyzer;

impl FileAnalyzer for MarkdownAnalyzer {
    fn analyze(&self, _file: &str, staged_file: &StagedFile) -> Vec<String> {
        let mut analysis = Vec::new();

        if let Some(headers) = extract_modified_headers(&staged_file.diff) {
            analysis.push(format!("Modified headers: {}", headers.join(", ")));
        }

        if has_list_changes(&staged_file.diff) {
            analysis.push("List structures have been modified".to_string());
        }

        if has_code_block_changes(&staged_file.diff) {
            analysis.push("Code blocks have been modified".to_string());
        }

        if has_link_changes(&staged_file.diff) {
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
