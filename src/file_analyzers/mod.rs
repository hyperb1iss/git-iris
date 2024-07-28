use crate::git::FileChange;

pub trait FileAnalyzer {
    fn analyze(&self, file: &str, change: &FileChange) -> Vec<String>;
    fn get_file_type(&self) -> &'static str;
}

mod rust;
mod toml;
// Add more file type modules here as we implement them

pub fn get_analyzer(file: &str) -> Box<dyn FileAnalyzer> {
    if file.ends_with(".rs") {
        Box::new(rust::RustAnalyzer)
    } else if file.ends_with(".toml") {
        Box::new(toml::TomlAnalyzer)
    } else {
        Box::new(DefaultAnalyzer)
    }
}

struct DefaultAnalyzer;

impl FileAnalyzer for DefaultAnalyzer {
    fn analyze(&self, _file: &str, _change: &FileChange) -> Vec<String> {
        vec![]
    }

    fn get_file_type(&self) -> &'static str {
        "Unknown file type"
    }
}