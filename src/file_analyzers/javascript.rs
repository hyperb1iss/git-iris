use super::FileAnalyzer;
use crate::git::FileChange;
use regex::Regex;
use std::collections::HashSet;

pub struct JavaScriptAnalyzer;

impl FileAnalyzer for JavaScriptAnalyzer {
    fn analyze(&self, _file: &str, change: &FileChange) -> Vec<String> {
        let mut analysis = Vec::new();

        if let Some(functions) = extract_modified_functions(&change.diff) {
            println!("JavaScript Debug: Detected functions: {:?}", functions);
            analysis.push(format!("Modified functions: {}", functions.join(", ")));
        }

        if let Some(classes) = extract_modified_classes(&change.diff) {
            analysis.push(format!("Modified classes: {}", classes.join(", ")));
        }

        if has_import_changes(&change.diff) {
            analysis.push("Import statements have been modified".to_string());
        }

        if let Some(components) = extract_modified_react_components(&change.diff) {
            println!(
                "JavaScript Debug: Detected React components: {:?}",
                components
            );
            analysis.push(format!(
                "Modified React components: {}",
                components.join(", ")
            ));
        }

        println!("JavaScript Debug: Final analysis: {:?}", analysis);
        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "JavaScript/TypeScript source file"
    }
}

fn extract_modified_functions(diff: &str) -> Option<Vec<String>> {
    let re = Regex::new(
        r"(?m)^[+-]\s*(function\s+(\w+)|const\s+(\w+)\s*=\s*(\([^)]*\)\s*=>|\function))",
    )
    .unwrap();
    let functions: Vec<String> = re
        .captures_iter(diff)
        .filter_map(|cap| cap.get(2).or(cap.get(3)).map(|m| m.as_str().to_string()))
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
    let re = Regex::new(r"(?m)^[+-]\s*(import|export)").unwrap();
    re.is_match(diff)
}

fn extract_modified_react_components(diff: &str) -> Option<Vec<String>> {
    let class_re = Regex::new(r"(?m)^[+-]\s*class\s+(\w+)\s+extends\s+React\.Component").unwrap();
    let func_re = Regex::new(r"(?m)^[+-]\s*(?:function\s+(\w+)|const\s+(\w+)\s*=)(?:\s*\([^)]*\))?\s*(?:=>)?\s*(?:\{[^}]*return|=>)\s*(?:<|\()").unwrap();

    let mut components = HashSet::new();

    for cap in class_re.captures_iter(diff) {
        if let Some(m) = cap.get(1) {
            components.insert(m.as_str().to_string());
        }
    }

    for cap in func_re.captures_iter(diff) {
        if let Some(m) = cap.get(1).or(cap.get(2)) {
            components.insert(m.as_str().to_string());
        }
    }

    if components.is_empty() {
        None
    } else {
        Some(components.into_iter().collect())
    }
}
