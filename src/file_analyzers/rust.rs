use super::FileAnalyzer;
use crate::git::FileChange;
use regex::Regex;

/// Analyzer for Rust source files
pub struct RustAnalyzer;

impl FileAnalyzer for RustAnalyzer {
    fn analyze(&self, _file: &str, change: &FileChange) -> Vec<String> {
        let mut analysis = Vec::new();

        // Extract modified functions from the diff
        if let Some(functions) = extract_modified_functions(&change.diff) {
            analysis.push(format!("Modified functions: {}", functions.join(", ")));
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "Rust source file"
    }
}

/// Extract modified function names from a diff string
fn extract_modified_functions(diff: &str) -> Option<Vec<String>> {
    let re = Regex::new(r"[+-]\s*(?:pub\s+)?fn\s+(\w+)").unwrap();
    let functions: Vec<String> = re
        .captures_iter(diff)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    if functions.is_empty() {
        None
    } else {
        Some(functions)
    }
}
