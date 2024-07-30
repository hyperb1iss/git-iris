use super::FileAnalyzer;
use crate::context::StagedFile;
use regex::Regex;

pub struct PythonAnalyzer;

impl FileAnalyzer for PythonAnalyzer {
    fn analyze(&self, _file: &str, staged_file: &StagedFile) -> Vec<String> {
        let mut analysis = Vec::new();

        if let Some(functions) = extract_modified_functions(&staged_file.diff) {
            analysis.push(format!("Modified functions: {}", functions.join(", ")));
        }

        if let Some(classes) = extract_modified_classes(&staged_file.diff) {
            analysis.push(format!("Modified classes: {}", classes.join(", ")));
        }

        if has_import_changes(&staged_file.diff) {
            analysis.push("Import statements have been modified".to_string());
        }

        if let Some(decorators) = extract_modified_decorators(&staged_file.diff) {
            analysis.push(format!("Modified decorators: {}", decorators.join(", ")));
        }

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "Python source file"
    }
}

fn extract_modified_functions(diff: &str) -> Option<Vec<String>> {
    let re = Regex::new(r"(?m)^[+-](?:(?:\s*@\w+\s*\n)+)?\s*def\s+(\w+)").unwrap();
    let functions: Vec<String> = re
        .captures_iter(diff)
        .filter_map(|cap| {
            let func_name = cap.get(1).map(|m| m.as_str().to_string())?;
            if func_name != "__init__" {
                Some(func_name)
            } else {
                None
            }
        })
        .collect();

    if functions.is_empty() {
        None
    } else {
        Some(functions)
    }
}

fn extract_modified_classes(diff: &str) -> Option<Vec<String>> {
    let re = Regex::new(r"(?m)^[+-]\s*class\s+(\w+)").unwrap();
    let classes: Vec<String> = re
        .captures_iter(diff)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    if classes.is_empty() {
        None
    } else {
        Some(classes)
    }
}

fn has_import_changes(diff: &str) -> bool {
    let re = Regex::new(r"(?m)^[+-]\s*(import|from)").unwrap();
    re.is_match(diff)
}

fn extract_modified_decorators(diff: &str) -> Option<Vec<String>> {
    let re = Regex::new(r"(?m)^[+-]\s*@(\w+)").unwrap();
    let decorators: Vec<String> = re
        .captures_iter(diff)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    if decorators.is_empty() {
        None
    } else {
        Some(decorators)
    }
}
