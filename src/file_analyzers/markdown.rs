use super::{FileAnalyzer, ProjectMetadata};
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

    fn extract_metadata(&self, file: &str, content: &str) -> ProjectMetadata {
        let mut metadata = ProjectMetadata::default();

        if file.to_lowercase() == "readme.md" {
            self.extract_readme_metadata(content, &mut metadata);
        }

        metadata
    }
}

impl MarkdownAnalyzer {
    fn extract_readme_metadata(&self, content: &str, metadata: &mut ProjectMetadata) {
        // Extract project name from the first header
        let title_re = Regex::new(r"(?m)^#\s+(.+)$").unwrap();
        if let Some(cap) = title_re.captures(content) {
            metadata.language = Some(cap[1].to_string());
        }

        // Look for badges that might indicate the build system or test framework
        if content.contains("travis-ci.org") {
            metadata.build_system = Some("Travis CI".to_string());
        } else if content.contains("github.com/actions/workflows") {
            metadata.build_system = Some("GitHub Actions".to_string());
        }

        if content.contains("coveralls.io") {
            metadata.test_framework = Some("Coveralls".to_string());
        }

        // Extract version if present
        let version_re = Regex::new(r"(?i)version[:\s]+(\d+\.\d+\.\d+)").unwrap();
        if let Some(cap) = version_re.captures(content) {
            metadata.version = Some(cap[1].to_string());
        }
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