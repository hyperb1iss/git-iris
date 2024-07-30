use super::{FileAnalyzer, ProjectMetadata};
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

    fn extract_metadata(&self, file: &str, content: &str) -> ProjectMetadata {
        let mut metadata = ProjectMetadata::default();

        if file == "package.json" {
            self.extract_package_json_metadata(content, &mut metadata);
        } else if file == "tsconfig.json" {
            metadata.language = Some("TypeScript".to_string());
        }

        metadata
    }
}

impl JsonAnalyzer {
    fn extract_package_json_metadata(&self, content: &str, metadata: &mut ProjectMetadata) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(version) = json["version"].as_str() {
                metadata.version = Some(version.to_string());
            }

            if let Some(dependencies) = json["dependencies"].as_object() {
                for dep in dependencies.keys() {
                    metadata.dependencies.push(dep.to_string());
                }
            }

            if let Some(dev_dependencies) = json["devDependencies"].as_object() {
                for dep in dev_dependencies.keys() {
                    metadata.dependencies.push(dep.to_string());
                }
            }

            metadata.language = Some("JavaScript".to_string());
            metadata.build_system = Some("npm".to_string());

            // Detect framework
            if json["dependencies"].get("react").is_some() {
                metadata.framework = Some("React".to_string());
            } else if json["dependencies"].get("vue").is_some() {
                metadata.framework = Some("Vue".to_string());
            } else if json["dependencies"].get("@angular/core").is_some() {
                metadata.framework = Some("Angular".to_string());
            }

            // Detect test framework
            if json["devDependencies"].get("jest").is_some() {
                metadata.test_framework = Some("Jest".to_string());
            } else if json["devDependencies"].get("mocha").is_some() {
                metadata.test_framework = Some("Mocha".to_string());
            }
        }
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