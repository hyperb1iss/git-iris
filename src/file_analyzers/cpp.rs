use super::{FileAnalyzer, ProjectMetadata};
use crate::context::StagedFile;
use regex::Regex;
use std::collections::HashSet;

pub struct CppAnalyzer;

impl FileAnalyzer for CppAnalyzer {
    fn analyze(&self, _file: &str, staged_file: &StagedFile) -> Vec<String> {
        let mut analysis = Vec::new();

        if let Some(functions) = extract_modified_functions(&staged_file.diff) {
            analysis.push(format!("Modified functions: {}", functions.join(", ")));
        }

        if let Some(classes) = extract_modified_classes(&staged_file.diff) {
            analysis.push(format!("Modified classes: {}", classes.join(", ")));
        }

        if has_include_changes(&staged_file.diff) {
            analysis.push("Include statements have been modified".to_string());
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "C++ source file"
    }

    fn extract_metadata(&self, file: &str, content: &str) -> ProjectMetadata {
        let mut metadata = ProjectMetadata::default();
        metadata.language = Some("C++".to_string());

        if file == "CMakeLists.txt" {
            self.extract_cmake_metadata(content, &mut metadata);
        } else {
            self.extract_cpp_file_metadata(content, &mut metadata);
        }

        metadata
    }
}

impl CppAnalyzer {
    fn extract_cmake_metadata(&self, content: &str, metadata: &mut ProjectMetadata) {
        metadata.build_system = Some("CMake".to_string());

        let version_re = Regex::new(r"project\([^)]+\s+VERSION\s+([^\s)]+)").unwrap();
        if let Some(cap) = version_re.captures(content) {
            metadata.version = Some(cap[1].to_string());
        }

        let dependency_re = Regex::new(r"find_package\(([^)]+)\)").unwrap();
        for cap in dependency_re.captures_iter(content) {
            let package = cap[1].split(' ').next().unwrap_or(&cap[1]);
            metadata.dependencies.push(package.to_string());
        }
    }

    fn extract_cpp_file_metadata(&self, content: &str, metadata: &mut ProjectMetadata) {
        if content.contains("#include <iostream>") {
            metadata.framework = Some("Standard I/O".to_string());
        }

        if content.contains("#include <vector>") {
            metadata.framework = Some("Standard Library".to_string());
        }
    }
}

fn extract_modified_functions(diff: &str) -> Option<Vec<String>> {
    let re = Regex::new(r"(?m)^[+-]\s*(?:static\s+)?(?:inline\s+)?(?:const\s+)?(?:volatile\s+)?(?:unsigned\s+)?(?:signed\s+)?(?:short\s+)?(?:long\s+)?(?:void|int|char|float|double|struct\s+\w+|class\s+\w+)\s+(\w+)\s*\(").unwrap();
    let functions: HashSet<String> = re
        .captures_iter(diff)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    if functions.is_empty() {
        None
    } else {
        Some(functions.into_iter().collect())
    }
}

fn extract_modified_classes(diff: &str) -> Option<Vec<String>> {
    let re = Regex::new(r"(?m)^[+-]\s*class\s+(\w+)").unwrap();
    let classes: HashSet<String> = re
        .captures_iter(diff)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    if classes.is_empty() {
        None
    } else {
        Some(classes.into_iter().collect())
    }
}

fn has_include_changes(diff: &str) -> bool {
    let re = Regex::new(r"(?m)^[+-]\s*#include").unwrap();
    re.is_match(diff)
}
