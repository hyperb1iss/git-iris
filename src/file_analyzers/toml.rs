use super::FileAnalyzer;
use crate::git::FileChange;

/// Analyzer for TOML configuration files
pub struct TomlAnalyzer;

impl FileAnalyzer for TomlAnalyzer {
    fn analyze(&self, file: &str, change: &FileChange) -> Vec<String> {
        let mut analysis = Vec::new();

        // Check for dependency changes in Cargo.toml
        if file.ends_with("Cargo.toml") && has_dependency_changes(&change.diff) {
            analysis.push("Dependencies updated".to_string());
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "TOML configuration file"
    }
}

/// Check if the diff contains dependency changes
fn has_dependency_changes(diff: &str) -> bool {
    diff.contains("[dependencies]") || diff.contains("version =")
}
