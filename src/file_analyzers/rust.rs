use super::FileAnalyzer;
use crate::git::FileChange;
use regex::Regex;

/// Analyzer for Rust source files
pub struct RustAnalyzer;

impl FileAnalyzer for RustAnalyzer {
    fn analyze(&self, _file: &str, change: &FileChange) -> Vec<String> {
        let mut analysis = Vec::new();

        // Check for new or modified functions
        if let Some(functions) = extract_modified_functions(&change.diff) {
            analysis.push(format!("Modified functions: {}", functions.join(", ")));
        }

        // Check for new or modified structs
        if let Some(structs) = extract_modified_structs(&change.diff) {
            analysis.push(format!("Modified structs: {}", structs.join(", ")));
        }

        // Check for new or modified traits
        if let Some(traits) = extract_modified_traits(&change.diff) {
            analysis.push(format!("Modified traits: {}", traits.join(", ")));
        }

        // Check for new or modified imports
        if has_import_changes(&change.diff) {
            analysis.push("Import statements have been modified".to_string());
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "Rust source file"
    }
}

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

fn extract_modified_structs(diff: &str) -> Option<Vec<String>> {
    let re = Regex::new(r"[+-]\s*(?:pub\s+)?struct\s+(\w+)").unwrap();
    let structs: Vec<String> = re
        .captures_iter(diff)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    if structs.is_empty() {
        None
    } else {
        Some(structs)
    }
}

fn extract_modified_traits(diff: &str) -> Option<Vec<String>> {
    let re = Regex::new(r"[+-]\s*(?:pub\s+)?trait\s+(\w+)").unwrap();
    let traits: Vec<String> = re
        .captures_iter(diff)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    if traits.is_empty() {
        None
    } else {
        Some(traits)
    }
}

fn has_import_changes(diff: &str) -> bool {
    let re = Regex::new(r"[+-]\s*(use|extern crate)").unwrap();
    re.is_match(diff)
}
