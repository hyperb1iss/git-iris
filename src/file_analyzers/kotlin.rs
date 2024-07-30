use super::{FileAnalyzer, ProjectMetadata};
use crate::context::StagedFile;
use regex::Regex;
use std::collections::HashSet;

pub struct KotlinAnalyzer;

impl FileAnalyzer for KotlinAnalyzer {
    fn analyze(&self, _file: &str, staged_file: &StagedFile) -> Vec<String> {
        let mut analysis = Vec::new();

        if let Some(classes) = extract_modified_classes(&staged_file.diff) {
            analysis.push(format!("Modified classes: {}", classes.join(", ")));
        }

        if let Some(functions) = extract_modified_functions(&staged_file.diff) {
            analysis.push(format!("Modified functions: {}", functions.join(", ")));
        }

        if has_import_changes(&staged_file.diff) {
            analysis.push("Import statements have been modified".to_string());
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "Kotlin source file"
    }

    fn extract_metadata(&self, file: &str, content: &str) -> ProjectMetadata {
        let mut metadata = ProjectMetadata::default();
        metadata.language = Some("Kotlin".to_string());

        if file == "build.gradle.kts" {
            self.extract_gradle_metadata(content, &mut metadata);
        } else {
            self.extract_kotlin_file_metadata(content, &mut metadata);
        }

        metadata
    }
}

impl KotlinAnalyzer {
    fn extract_gradle_metadata(&self, content: &str, metadata: &mut ProjectMetadata) {
        metadata.build_system = Some("Gradle".to_string());

        let version_re = Regex::new(r#"version\s*=\s*['"](.*?)['"]"#).unwrap();
        if let Some(cap) = version_re.captures(content) {
            metadata.version = Some(cap[1].to_string());
        }

        let dependency_re = Regex::new(r#"implementation\s*\(\s*["'](.+?):(.+?):(.+?)["']\)"#).unwrap();
        for cap in dependency_re.captures_iter(content) {
            metadata.dependencies.push(format!("{}:{}:{}", &cap[1], &cap[2], &cap[3]));
        }
    }

    fn extract_kotlin_file_metadata(&self, content: &str, metadata: &mut ProjectMetadata) {
        if content.contains("import org.springframework") {
            metadata.framework = Some("Spring".to_string());
        } else if content.contains("import javax.ws.rs") {
            metadata.framework = Some("JAX-RS".to_string());
        }

        if content.contains("import org.junit.") {
            metadata.test_framework = Some("JUnit".to_string());
        } else if content.contains("import org.testng.") {
            metadata.test_framework = Some("TestNG".to_string());
        }
    }
}

fn extract_modified_classes(diff: &str) -> Option<Vec<String>> {
    let re = Regex::new(r"(?m)^[+-]\s*(class|interface|object)\s+(\w+)").unwrap();
    let classes: HashSet<String> = re
        .captures_iter(diff)
        .filter_map(|cap| cap.get(2).map(|m| m.as_str().to_string()))
        .collect();

    if classes.is_empty() {
        None
    } else {
        Some(classes.into_iter().collect())
    }
}

fn extract_modified_functions(diff: &str) -> Option<Vec<String>> {
    let re = Regex::new(r"(?m)^[+-]\s*(fun)\s+(\w+)").unwrap();
    let functions: HashSet<String> = re
        .captures_iter(diff)
        .filter_map(|cap| cap.get(2).map(|m| m.as_str().to_string()))
        .collect();

    if functions.is_empty() {
        None
    } else {
        Some(functions.into_iter().collect())
    }
}

fn has_import_changes(diff: &str) -> bool {
    let re = Regex::new(r"(?m)^[+-]\s*import\s+").unwrap();
    re.is_match(diff)
}