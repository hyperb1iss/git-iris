use super::{FileAnalyzer, ProjectMetadata};
use crate::context::StagedFile;
use regex::Regex;
use std::collections::HashSet;

pub struct JavaScriptAnalyzer;

impl FileAnalyzer for JavaScriptAnalyzer {
    fn analyze(&self, _file: &str, staged_file: &StagedFile) -> Vec<String> {
        let mut analysis = Vec::new();

        if let Some(functions) = extract_modified_functions(&staged_file.diff) {
            analysis.push(format!("Modified functions: {}", functions.join(", ")));
        }

        if let Some(classes) = extract_modified_classes(&staged_file.diff) {
            analysis.push(format!("Modified classes: {}", classes.join(", ")));
        }

        if has_import_changes(&staged_file.diff) {
            analysis.push("Import statements have been modified".to_string());
        }

        if let Some(components) = extract_modified_react_components(&staged_file.diff) {
            analysis.push(format!(
                "Modified React components: {}",
                components.join(", ")
            ));
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "JavaScript/TypeScript source file"
    }

    fn extract_metadata(&self, file: &str, content: &str) -> ProjectMetadata {
        let mut metadata = ProjectMetadata::default();
        metadata.language = Some(if file.ends_with(".ts") { "TypeScript" } else { "JavaScript" }.to_string());

        if file == "package.json" {
            self.extract_package_json_metadata(content, &mut metadata);
        } else {
            self.extract_js_file_metadata(content, &mut metadata);
        }

        metadata
    }
}

impl JavaScriptAnalyzer {
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
                    if dep.contains("test") || dep.contains("jest") || dep.contains("mocha") {
                        metadata.test_framework = Some(dep.to_string());
                        break;
                    }
                }
            }

            metadata.build_system = Some("npm".to_string());
        }
    }

    fn extract_js_file_metadata(&self, content: &str, metadata: &mut ProjectMetadata) {
        if content.contains("import React") || content.contains("from 'react'") {
            metadata.framework = Some("React".to_string());
        } else if content.contains("import Vue") || content.contains("from 'vue'") {
            metadata.framework = Some("Vue".to_string());
        } else if content.contains("import { Component") || content.contains("from '@angular/core'") {
            metadata.framework = Some("Angular".to_string());
        }
    }
}

fn extract_modified_functions(diff: &str) -> Option<Vec<String>> {
    let re = Regex::new(
        r"(?m)^[+-]\s*(function\s+(\w+)|const\s+(\w+)\s*=\s*(\([^)]*\)\s*=>|\function))",
    )
    .unwrap();
    let functions: Vec<String> = re
        .captures_iter(diff)
        .filter_map(|cap| cap.get(2).or(cap.get(3)).map(|m| m.as_str().to_string()))
        .collect();

    if functions.is_empty() {
        None
    } else {
        Some(functions)
    }
}

fn extract_modified_classes(diff: &str) -> Option<Vec<String>> {
    let re = Regex::new(r"(?m)^[+-]\s*class\s+(\w+)").unwrap();
    let classes: Vec<String> = re
        .captures_iter(diff)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    if classes.is_empty() {
        None
    } else {
        Some(classes)
    }
}

fn has_import_changes(diff: &str) -> bool {
    let re = Regex::new(r"(?m)^[+-]\s*(import|export)").unwrap();
    re.is_match(diff)
}

fn extract_modified_react_components(diff: &str) -> Option<Vec<String>> {
    let class_re = Regex::new(r"(?m)^[+-]\s*class\s+(\w+)\s+extends\s+React\.Component").unwrap();
    let func_re = Regex::new(r"(?m)^[+-]\s*(?:function\s+(\w+)|const\s+(\w+)\s*=)(?:\s*\([^)]*\))?\s*(?:=>)?\s*(?:\{[^}]*return|=>)\s*(?:<|\()").unwrap();

    let mut components = HashSet::new();

    for cap in class_re.captures_iter(diff) {
        if let Some(m) = cap.get(1) {
            components.insert(m.as_str().to_string());
        }
    }

    for cap in func_re.captures_iter(diff) {
        if let Some(m) = cap.get(1).or(cap.get(2)) {
            components.insert(m.as_str().to_string());
        }
    }

    if components.is_empty() {
        None
    } else {
        Some(components.into_iter().collect())
    }
}