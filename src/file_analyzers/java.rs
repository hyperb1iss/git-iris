use super::FileAnalyzer;
use crate::context::StagedFile;
use regex::Regex;
use std::collections::HashSet;

pub struct JavaAnalyzer;

impl FileAnalyzer for JavaAnalyzer {
    fn analyze(&self, _file: &str, staged_file: &StagedFile) -> Vec<String> {
        let mut analysis = Vec::new();

        if let Some(classes) = extract_modified_classes(&staged_file.diff) {
            analysis.push(format!("Modified classes: {}", classes.join(", ")));
        }

        if let Some(methods) = extract_modified_methods(&staged_file.diff) {
            analysis.push(format!("Modified methods: {}", methods.join(", ")));
        }

        if has_import_changes(&staged_file.diff) {
            analysis.push("Import statements have been modified".to_string());
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "Java source file"
    }
}

fn extract_modified_classes(diff: &str) -> Option<Vec<String>> {
    let re =
        Regex::new(r"(?m)^[+-]\s*(public\s+|private\s+)?(class|interface|enum)\s+(\w+)").unwrap();
    let classes: HashSet<String> = re
        .captures_iter(diff)
        .filter_map(|cap| cap.get(3).map(|m| m.as_str().to_string()))
        .collect();

    if classes.is_empty() {
        None
    } else {
        Some(classes.into_iter().collect())
    }
}

fn extract_modified_methods(diff: &str) -> Option<Vec<String>> {
    let re =
        Regex::new(r"(?m)^[+-]\s*(public|protected|private)?\s*\w+\s+(\w+)\s*\([^\)]*\)").unwrap();
    let methods: HashSet<String> = re
        .captures_iter(diff)
        .filter_map(|cap| cap.get(2).map(|m| m.as_str().to_string()))
        .collect();

    if methods.is_empty() {
        None
    } else {
        Some(methods.into_iter().collect())
    }
}

fn has_import_changes(diff: &str) -> bool {
    let re = Regex::new(r"(?m)^[+-]\s*import\s+").unwrap();
    re.is_match(diff)
}
