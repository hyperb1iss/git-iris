use super::FileAnalyzer;
use crate::context::StagedFile;
use regex::Regex;

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
