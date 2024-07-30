use super::FileAnalyzer;
use crate::context::StagedFile;
use regex::Regex;
use std::collections::HashSet;

pub struct JsonAnalyzer;

impl FileAnalyzer for JsonAnalyzer {
    fn analyze(&self, _file: &str, staged_file: &StagedFile) -> Vec<String> {
        let mut analysis = Vec::new();

        if let Some(keys) = extract_modified_top_level_keys(&staged_file.diff) {
            analysis.push(format!("Modified top-level keys: {}", keys.join(", ")));
        }

        if has_array_changes(&staged_file.diff) {
            analysis.push("Array structures have been modified".to_string());
        }

        if has_nested_object_changes(&staged_file.diff) {
            analysis.push("Nested objects have been modified".to_string());
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "JSON configuration file"
    }
}

fn extract_modified_top_level_keys(diff: &str) -> Option<Vec<String>> {
    let lines: Vec<&str> = diff.lines().collect();
    let re = Regex::new(r#"^[+-]\s*"(\w+)"\s*:"#).unwrap();
    let mut keys = HashSet::new();

    for (i, line) in lines.iter().enumerate() {
        if let Some(cap) = re.captures(line) {
            let key = cap.get(1).unwrap().as_str();
            let prev_line = if i > 0 { lines[i - 1] } else { "" };
            let next_line = lines.get(i + 1).unwrap_or(&"");

            if !prev_line.trim().ends_with("{") && !next_line.trim().starts_with("}") {
                keys.insert(key.to_string());
            }
        }
    }

    if keys.is_empty() {
        None
    } else {
        Some(keys.into_iter().collect())
    }
}

fn has_array_changes(diff: &str) -> bool {
    let re = Regex::new(r#"(?m)^[+-]\s*(?:"[^"]+"\s*:\s*)?\[|\s*[+-]\s*"[^"]+","#).unwrap();
    re.is_match(diff)
}

fn has_nested_object_changes(diff: &str) -> bool {
    let re = Regex::new(r#"(?m)^[+-]\s*"[^"]+"\s*:\s*\{|\s*[+-]\s*"[^"]+"\s*:"#).unwrap();
    re.is_match(diff)
}
