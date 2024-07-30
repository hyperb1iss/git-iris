use super::FileAnalyzer;
use crate::context::StagedFile;
use regex::Regex;
use std::collections::HashSet;

pub struct YamlAnalyzer;

impl FileAnalyzer for YamlAnalyzer {
    fn analyze(&self, _file: &str, staged_file: &StagedFile) -> Vec<String> {
        let mut analysis = Vec::new();

        if let Some(keys) = extract_modified_top_level_keys(&staged_file.diff) {
            analysis.push(format!("Modified top-level keys: {}", keys.join(", ")));
        }

        if has_list_changes(&staged_file.diff) {
            analysis.push("List structures have been modified".to_string());
        }

        if has_nested_changes(&staged_file.diff) {
            analysis.push("Nested structures have been modified".to_string());
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "YAML configuration file"
    }
}

fn extract_modified_top_level_keys(diff: &str) -> Option<Vec<String>> {
    let re = Regex::new(r"(?m)^[+-]\s*(\w+):(?:\s|$)").unwrap();
    let keys: HashSet<String> = re
        .captures_iter(diff)
        .filter_map(|cap| {
            let key = cap.get(1).map(|m| m.as_str().to_string())?;
            if !diff.contains(&format!("  {}", key)) {
                Some(key)
            } else {
                None
            }
        })
        .collect();

    if keys.is_empty() {
        None
    } else {
        Some(keys.into_iter().collect())
    }
}

fn has_list_changes(diff: &str) -> bool {
    let re = Regex::new(r"(?m)^[+-]\s*-\s+").unwrap();
    re.is_match(diff)
}

fn has_nested_changes(diff: &str) -> bool {
    let re = Regex::new(r"(?m)^[+-]\s+\w+:").unwrap();
    re.is_match(diff)
}
