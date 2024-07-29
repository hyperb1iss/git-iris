use crate::git::FileChange;

/// Trait for analyzing files and extracting relevant information
pub trait FileAnalyzer {
    /// Analyze the file and return a list of analysis results
    fn analyze(&self, file: &str, change: &FileChange) -> Vec<String>;
    /// Get the type of the file being analyzed
    fn get_file_type(&self) -> &'static str;
}

/// Module for analyzing Rust files
mod rust;
/// Module for analyzing TOML files
mod toml;
// Add more file type modules here as we implement them

/// Get the appropriate file analyzer based on the file extension
pub fn get_analyzer(file: &str) -> Box<dyn FileAnalyzer> {
    if file.ends_with(".rs") {
        Box::new(rust::RustAnalyzer)
    } else if file.ends_with(".toml") {
        Box::new(toml::TomlAnalyzer)
    } else {
        Box::new(DefaultAnalyzer)
    }
}

/// Default analyzer for unsupported file types
struct DefaultAnalyzer;

impl FileAnalyzer for DefaultAnalyzer {
    fn analyze(&self, _file: &str, _change: &FileChange) -> Vec<String> {
        vec![]
    }

    fn get_file_type(&self) -> &'static str {
        "Unknown file type"
    }
}
