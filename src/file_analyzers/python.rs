use super::{FileAnalyzer, ProjectMetadata};
use crate::context::StagedFile;
use regex::Regex;

pub struct PythonAnalyzer;

impl FileAnalyzer for PythonAnalyzer {
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

        if let Some(decorators) = extract_modified_decorators(&staged_file.diff) {
            analysis.push(format!("Modified decorators: {}", decorators.join(", ")));
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "Python source file"
    }

    fn extract_metadata(&self, file: &str, content: &str) -> ProjectMetadata {
        let mut metadata = ProjectMetadata {
            language: Some("Python".to_string()),
            ..Default::default()
        };

        if file == "requirements.txt" {
            Self::extract_requirements_metadata(content, &mut metadata);
        } else if file == "setup.py" {
            Self::extract_setup_metadata(content, &mut metadata);
        } else {
            Self::extract_py_file_metadata(content, &mut metadata);
        }

        metadata
    }
}

impl PythonAnalyzer {
    fn extract_requirements_metadata(content: &str, metadata: &mut ProjectMetadata) {
        for line in content.lines() {
            let package = line.split('=').next().unwrap_or(line).trim();
            if !package.is_empty() && !package.starts_with('#') {
                metadata.dependencies.push(package.to_string());
            }
        }
    }

    fn extract_setup_metadata(content: &str, metadata: &mut ProjectMetadata) {
        let version_re =
            Regex::new(r#"version\s*=\s*['"]([^'"]+)['"]"#).expect("Could not compile regex");
        if let Some(cap) = version_re.captures(content) {
            metadata.version = Some(cap[1].to_string());
        }

        let install_requires_re =
            Regex::new(r"install_requires\s*=\s*\[(.*?)\]").expect("Could not compile regex");
        if let Some(cap) = install_requires_re.captures(content) {
            let deps = cap[1].split(',');
            for dep in deps {
                let cleaned = dep.trim().trim_matches(|c| c == '\'' || c == '"');
                if !cleaned.is_empty() {
                    metadata.dependencies.push(cleaned.to_string());
                }
            }
        }
    }

    fn extract_py_file_metadata(content: &str, metadata: &mut ProjectMetadata) {
        if content.contains("import django") || content.contains("from django") {
            metadata.framework = Some("Django".to_string());
        } else if content.contains("import flask") || content.contains("from flask") {
            metadata.framework = Some("Flask".to_string());
        }

        if content.contains("import pytest") || content.contains("from pytest") {
            metadata.test_framework = Some("pytest".to_string());
        } else if content.contains("import unittest") || content.contains("from unittest") {
            metadata.test_framework = Some("unittest".to_string());
        }
    }
}

fn extract_modified_functions(diff: &str) -> Option<Vec<String>> {
    let re = Regex::new(r"(?m)^[+-](?:(?:\s*@\w+\s*\n)+)?\s*def\s+(\w+)")
        .expect("Could not compile regex");
    let functions: Vec<String> = re
        .captures_iter(diff)
        .filter_map(|cap| {
            let func_name = cap.get(1).map(|m| m.as_str().to_string())?;
            if func_name == "__init__" {
                None
            } else {
                Some(func_name)
            }
        })
        .collect();

    if functions.is_empty() {
        None
    } else {
        Some(functions)
    }
}

fn extract_modified_classes(diff: &str) -> Option<Vec<String>> {
    let re = Regex::new(r"(?m)^[+-]\s*class\s+(\w+)").expect("Could not compile regex");
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
    let re = Regex::new(r"(?m)^[+-]\s*(import|from)").expect("Could not compile regex");
    re.is_match(diff)
}

fn extract_modified_decorators(diff: &str) -> Option<Vec<String>> {
    let re = Regex::new(r"(?m)^[+-]\s*@(\w+)").expect("Could not compile regex");
    let decorators: Vec<String> = re
        .captures_iter(diff)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    if decorators.is_empty() {
        None
    } else {
        Some(decorators)
    }
}
