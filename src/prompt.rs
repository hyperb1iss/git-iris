use anyhow::Result;
use crate::git::{GitInfo, FileChange};
use crate::config::Config;
use std::path::Path;
use regex::Regex;

pub fn create_prompt(git_info: &GitInfo, config: &Config, verbose: bool) -> Result<String> {
    let base_prompt = if config.use_gitmoji {
        "Generate a Git commit message using gitmoji based on the following information:"
    } else {
        "Generate a Git commit message based on the following information:"
    };

    let context_prompt = format!(
        "Project root: {}\n
Current branch: {}\n
Recent commits:\n{}\n
Staged changes:\n{}\n
Unstaged files:\n{}\n
Change analysis:\n{}",
        git_info.project_root,
        git_info.branch,
        format_recent_commits(&git_info.recent_commits),
        format_staged_files(&git_info.staged_files),
        git_info.unstaged_files.join("\n"),
        analyze_changes(&git_info.staged_files, &git_info.project_root)
    );

    let full_prompt = format!(
        "{}\n\nContext:\n{}\n\nBased on this information, generate an appropriate commit message. Focus on the overall purpose of the changes, and include specific details only if they are significant. Do not use backticks around filenames, and avoid mentioning minor changes like individual import additions.",
        base_prompt, context_prompt
    );

    if verbose {
        println!("Prompt being sent to LLM:\n{}", full_prompt);
    }

    Ok(full_prompt)
}

fn format_recent_commits(commits: &[String]) -> String {
    commits.iter()
        .map(|commit| {
            if let Some(ticket) = extract_ticket_number(commit) {
                format!("{} (Ticket: {})", commit, ticket)
            } else {
                commit.to_string()
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn extract_ticket_number(commit: &str) -> Option<String> {
    let re = Regex::new(r"(TICKET-\d+)").unwrap();
    re.captures(commit)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
}

fn format_staged_files(staged_files: &std::collections::HashMap<String, FileChange>) -> String {
    staged_files
        .iter()
        .map(|(file, change)| format!(
            "File: {} ({}) - {}\nDiff:\n{}\n",
            file,
            format_file_status(&change.status),
            get_file_type(file),
            truncate_diff(&change.diff, 500)
        ))
        .collect::<Vec<String>>()
        .join("\n")
}

fn format_file_status(status: &str) -> &str {
    match status {
        "A" => "Added",
        "M" => "Modified",
        "D" => "Deleted",
        _ => "Changed",
    }
}

fn get_file_type(file_path: &str) -> String {
    let extension = Path::new(file_path)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("unknown");

    match extension {
        "rs" => "Rust source file".to_string(),
        "toml" => "TOML configuration file".to_string(),
        "md" => "Markdown documentation".to_string(),
        "json" => "JSON data file".to_string(),
        "yml" | "yaml" => "YAML configuration file".to_string(),
        _ => format!("File with .{} extension", extension),
    }
}

fn truncate_diff(diff: &str, max_length: usize) -> String {
    if diff.len() > max_length {
        format!("{}... (truncated)", &diff[..max_length])
    } else {
        diff.to_string()
    }
}

fn analyze_changes(staged_files: &std::collections::HashMap<String, FileChange>, project_root: &str) -> String {
    let mut analysis = Vec::new();

    for (file, change) in staged_files {
        let relative_path = file.strip_prefix(project_root).unwrap_or(file);
        let file_type = get_file_type(file);

        if file_type == "Rust source file" {
            if let Some(functions) = extract_modified_functions(&change.diff) {
                analysis.push(format!("Modified functions in {}: {}", relative_path, functions.join(", ")));
            }
        } else if file_type == "TOML configuration file" && file.ends_with("Cargo.toml") {
            if has_dependency_changes(&change.diff) {
                analysis.push(format!("Dependencies updated in {}", relative_path));
            }
        }

        // Add more file-specific analyses here as needed
    }

    if analysis.is_empty() {
        "No significant patterns detected in the changes.".to_string()
    } else {
        analysis.join("\n")
    }
}

fn extract_modified_functions(diff: &str) -> Option<Vec<String>> {
    let re = Regex::new(r"[+-]\s*(?:pub\s+)?fn\s+(\w+)").unwrap();
    let functions: Vec<String> = re.captures_iter(diff)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    if functions.is_empty() {
        None
    } else {
        Some(functions)
    }
}

fn has_dependency_changes(diff: &str) -> bool {
    diff.contains("[dependencies]") || diff.contains("version =")
}