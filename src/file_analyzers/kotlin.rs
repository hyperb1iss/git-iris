use super::FileAnalyzer;
use crate::git::FileChange;
use regex::Regex;

pub struct KotlinAnalyzer;

impl FileAnalyzer for KotlinAnalyzer {
    fn analyze(&self, _file: &str, change: &FileChange) -> Vec<String> {
        let mut analysis = Vec::new();

        if let Some(classes) = extract_modified_classes(&change.diff) {
            analysis.push(format!("Modified classes: {}", classes.join(", ")));
        }

        if let Some(functions) = extract_modified_functions(&change.diff) {
            analysis.push(format!("Modified functions: {}", functions.join(", ")));
        }

        if has_import_changes(&change.diff) {
            analysis.push("Import statements have been modified".to_string());
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "Kotlin source file"
    }
}

fn extract_modified_classes(diff: &str) -> Option<Vec<String>> {
    let re = Regex::new(r"(?m)^[+-]\s*(class|interface|object)\s+(\w+)").unwrap();
    let classes: Vec<String> = re
        .captures_iter(diff)
        .filter_map(|cap| cap.get(2).map(|m| m.as_str().to_string()))
        .collect();

    if classes.is_empty() {
        None
    } else {
        Some(classes)
    }
}

fn extract_modified_functions(diff: &str) -> Option<Vec<String>> {
    let re = Regex::new(r"(?m)^[+-]\s*(fun)\s+(\w+)").unwrap();
    let functions: Vec<String> = re
        .captures_iter(diff)
        .filter_map(|cap| cap.get(2).map(|m| m.as_str().to_string()))
        .collect();

    if functions.is_empty() {
        None
    } else {
        Some(functions)
    }
}

fn has_import_changes(diff: &str) -> bool {
    let re = Regex::new(r"(?m)^[+-]\s*import\s+").unwrap();
    re.is_match(diff)
}
