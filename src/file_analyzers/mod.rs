use regex::Regex;
use std::path::Path;

use crate::{
    context::{ProjectMetadata, StagedFile},
    log_debug,
};

/// Trait for analyzing files and extracting relevant information
pub trait FileAnalyzer: Send + Sync {
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
#[allow(clippy::case_sensitive_file_extension_comparisons)] // todo: check if we should compare case-insensitively
pub fn get_analyzer(file: &str) -> Box<dyn FileAnalyzer + Send + Sync> {
    if file.ends_with(".c") || file == "Makefile" {
        Box::new(c::CAnalyzer)
    } else if file.ends_with(".cpp")
        || file.ends_with(".cc")
        || file.ends_with(".cxx")
        || file == "CMakeLists.txt"
    {
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
        ProjectMetadata {
            language: Some("Unknown".to_string()),
            ..Default::default()
        }
    }
}

/// Checks if a file should be excluded from analysis.
///
/// # Arguments
///
/// * `path` - The path of the file to check.
///
/// # Returns
///
/// A boolean indicating whether the file should be excluded.
#[allow(clippy::unwrap_used)]
pub fn should_exclude_file(path: &str) -> bool {
    log_debug!("Checking if file should be excluded: {}", path);
    let exclude_patterns = vec![
        (String::from(r"\.git"), false),
        (String::from(r"\.svn"), false),
        (String::from(r"\.hg"), false),
        (String::from(r"\.DS_Store"), false),
        (String::from(r"node_modules"), false),
        (String::from(r"target"), false),
        (String::from(r"build"), false),
        (String::from(r"dist"), false),
        (String::from(r"\.vscode"), false),
        (String::from(r"\.idea"), false),
        (String::from(r"\.vs"), false),
        (String::from(r"package-lock\.json$"), true),
        (String::from(r"\.lock$"), true),
        (String::from(r"\.log$"), true),
        (String::from(r"\.tmp$"), true),
        (String::from(r"\.temp$"), true),
        (String::from(r"\.swp$"), true),
        (String::from(r"\.min\.js$"), true),
    ];

    let path = Path::new(path);

    for (pattern, is_extension) in exclude_patterns {
        let re = Regex::new(&pattern).expect("Could not compile regex");
        if is_extension {
            if let Some(file_name) = path.file_name() {
                if re.is_match(file_name.to_str().unwrap()) {
                    log_debug!("File excluded: {}", path.display());
                    return true;
                }
            }
        } else if re.is_match(path.to_str().unwrap()) {
            log_debug!("File excluded: {}", path.display());
            return true;
        }
    }
    log_debug!("File not excluded: {}", path.display());
    false
}
