use super::{FileAnalyzer, ProjectMetadata};
use crate::context::StagedFile;
use regex::Regex;
use std::collections::HashSet;

pub struct GradleAnalyzer;

impl FileAnalyzer for GradleAnalyzer {
    fn analyze(&self, _file: &str, staged_file: &StagedFile) -> Vec<String> {
        let mut analysis = Vec::new();

        if has_dependency_changes(&staged_file.diff) {
            analysis.push("Dependencies have been modified".to_string());
        }

        if has_plugin_changes(&staged_file.diff) {
            analysis.push("Plugins have been modified".to_string());
        }

        if has_task_changes(&staged_file.diff) {
            analysis.push("Tasks have been modified".to_string());
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "Gradle build file"
    }

    fn extract_metadata(&self, _file: &str, content: &str) -> ProjectMetadata {
        let mut metadata = ProjectMetadata::default();
        metadata.language = Some("Groovy/Kotlin".to_string());
        metadata.build_system = Some("Gradle".to_string());

        if let Some(version) = extract_gradle_version(content) {
            metadata.version = Some(version);
        }

        if let Some(dependencies) = extract_gradle_dependencies(content) {
            metadata.dependencies = dependencies;
        }

        if let Some(plugins) = extract_gradle_plugins(content) {
            metadata.plugins = plugins;
        }

        metadata
    }
}

fn has_dependency_changes(diff: &str) -> bool {
    let re = Regex::new(r"(?m)^[+-]\s*(implementation|api|testImplementation|compile)").unwrap();
    re.is_match(diff)
}

fn has_plugin_changes(diff: &str) -> bool {
    let re = Regex::new(r"(?m)^[+-]\s*(plugins|apply plugin)").unwrap();
    re.is_match(diff)
}

fn has_task_changes(diff: &str) -> bool {
    let re = Regex::new(r"(?m)^[+-]\s*task\s+").unwrap();
    re.is_match(diff)
}

fn extract_gradle_version(content: &str) -> Option<String> {
    let version_re = Regex::new(r#"version\s*=\s*['"](.*?)['"]"#).unwrap();
    version_re.captures(content).map(|cap| cap[1].to_string())
}

fn extract_gradle_dependencies(content: &str) -> Option<Vec<String>> {
    let dependency_re = Regex::new(r#"implementation\s+['"](.+?):(.+?):(.+?)['"]"#).unwrap();
    let dependencies: HashSet<String> = dependency_re
        .captures_iter(content)
        .map(|cap| format!("{}:{}:{}", &cap[1], &cap[2], &cap[3]))
        .collect();

    if dependencies.is_empty() {
        None
    } else {
        Some(dependencies.into_iter().collect())
    }
}

fn extract_gradle_plugins(content: &str) -> Option<Vec<String>> {
    let plugin_re = Regex::new(r#"id\s+['"](.+?)['"]"#).unwrap();
    let plugins: HashSet<String> = plugin_re
        .captures_iter(content)
        .map(|cap| cap[1].to_string())
        .collect();

    if plugins.is_empty() {
        None
    } else {
        Some(plugins.into_iter().collect())
    }
}
