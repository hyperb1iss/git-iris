use super::{FileAnalyzer, ProjectMetadata};
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

    fn extract_metadata(&self, file: &str, content: &str) -> ProjectMetadata {
        let mut metadata = ProjectMetadata::default();

        if file == "docker-compose.yml" || file == "docker-compose.yaml" {
            metadata.build_system = Some("Docker Compose".to_string());
        } else if file.ends_with(".github/workflows/ci.yml")
            || file.ends_with(".github/workflows/ci.yaml")
        {
            metadata.build_system = Some("GitHub Actions".to_string());
        } else if file == ".travis.yml" {
            metadata.build_system = Some("Travis CI".to_string());
        }

        // Extract version if present
        let version_re =
            Regex::new(r#"(?m)^version:\s*['"]?(.+?)['"]?$"#).expect("Could not compile regex");
        if let Some(cap) = version_re.captures(content) {
            metadata.version = Some(cap[1].to_string());
        }

        metadata
    }
}

fn extract_modified_top_level_keys(diff: &str) -> Option<Vec<String>> {
    let re = Regex::new(r"(?m)^[+-]\s*(\w+):(?:\s|$)").expect("Could not compile regex");
    let keys: HashSet<String> = re
        .captures_iter(diff)
        .filter_map(|cap| {
            let key = cap.get(1).map(|m| m.as_str().to_string())?;
            if diff.contains(&format!("  {key}")) {
                None
            } else {
                Some(key)
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
    let re = Regex::new(r"(?m)^[+-]\s*-\s+").expect("Could not compile regex");
    re.is_match(diff)
}

fn has_nested_changes(diff: &str) -> bool {
    let re = Regex::new(r"(?m)^[+-]\s+\w+:").expect("Could not compile regex");
    re.is_match(diff)
}
