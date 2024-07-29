use crate::git::FileChange;

/// Trait for analyzing files and extracting relevant information
pub trait FileAnalyzer {
    /// Analyze the file and return a list of analysis results
    fn analyze(&self, file: &str, change: &FileChange) -> Vec<String>;
    /// Get the type of the file being analyzed
    fn get_file_type(&self) -> &'static str;
}

/// Module for analyzing JavaScript files
mod javascript;
/// Module for analyzing JSON files
mod json;
/// Module for analyzing Markdown files
mod markdown;
/// Module for analyzing Python files
mod python;
/// Module for analyzing Rust files
mod rust;
/// Module for analyzing YAML files
mod yaml;

/// Get the appropriate file analyzer based on the file extension
pub fn get_analyzer(file: &str) -> Box<dyn FileAnalyzer> {
    if file.ends_with(".rs") {
        Box::new(rust::RustAnalyzer)
    } else if file.ends_with(".js") || file.ends_with(".ts") {
        Box::new(javascript::JavaScriptAnalyzer)
    } else if file.ends_with(".py") {
        Box::new(python::PythonAnalyzer)
    } else if file.ends_with(".yaml") || file.ends_with(".yml") {
        Box::new(yaml::YamlAnalyzer)
    } else if file.ends_with(".json") {
        Box::new(json::JsonAnalyzer)
    } else if file.ends_with(".md") {
        Box::new(markdown::MarkdownAnalyzer)
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
