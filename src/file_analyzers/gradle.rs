use super::FileAnalyzer;
use crate::git::FileChange;
use regex::Regex;

pub struct GradleAnalyzer;

impl FileAnalyzer for GradleAnalyzer {
    fn analyze(&self, _file: &str, change: &FileChange) -> Vec<String> {
        let mut analysis = Vec::new();

        if has_dependency_changes(&change.diff) {
            analysis.push("Dependencies have been modified".to_string());
        }

        if has_plugin_changes(&change.diff) {
            analysis.push("Plugins have been modified".to_string());
        }

        if has_task_changes(&change.diff) {
            analysis.push("Tasks have been modified".to_string());
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "Gradle build file"
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
