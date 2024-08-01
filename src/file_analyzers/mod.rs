use crate::context::{ProjectMetadata, StagedFile};

/// Trait for analyzing files and extracting relevant information
pub trait FileAnalyzer {
    fn analyze(&self, file: &str, staged_file: &StagedFile) -> Vec<String>;
    fn get_file_type(&self) -> &'static str;
    fn extract_metadata(&self, file: &str, content: &str) -> ProjectMetadata;
}

/// Module for analyzing C files
mod c;
/// Module for analyzing C++ files
mod cpp;
/// Module for analyzing Gradle files
mod gradle;
/// Module for analyzing Java files
mod java;
/// Module for analyzing JavaScript files
mod javascript;
/// Module for analyzing JSON files
mod json;
/// Module for analyzing Kotlin files
mod kotlin;
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
    if file.ends_with(".c") || file == "Makefile" {
        Box::new(c::CAnalyzer)
    } else if file.ends_with(".cpp") || file.ends_with(".cc") || file.ends_with(".cxx") ||  file == "CMakeLists.txt" {
        Box::new(cpp::CppAnalyzer)
    } else if file.ends_with(".rs") {
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
    } else if file.ends_with(".java") {
        Box::new(java::JavaAnalyzer)
    } else if file.ends_with(".kt") {
        Box::new(kotlin::KotlinAnalyzer)
    } else if file.ends_with(".gradle") || file.ends_with(".gradle.kts") {
        Box::new(gradle::GradleAnalyzer)
    } else {
        Box::new(DefaultAnalyzer)
    }
}

/// Default analyzer for unsupported file types
struct DefaultAnalyzer;

impl FileAnalyzer for DefaultAnalyzer {
    fn analyze(&self, _file: &str, _staged_file: &StagedFile) -> Vec<String> {
        vec![]
    }

    fn get_file_type(&self) -> &'static str {
        "Unknown file type"
    }

    fn extract_metadata(&self, _file: &str, _content: &str) -> ProjectMetadata {
        let mut metadata = ProjectMetadata::default();
        metadata.language = Some("Unknown".to_string());
        metadata
    }
}
