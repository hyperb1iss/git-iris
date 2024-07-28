use anyhow::Result;
use crate::git::{GitInfo, FileChange};

pub fn create_prompt(git_info: &GitInfo) -> Result<String> {
    let base_prompt = "You are an AI assistant helping to generate Git commit messages. 
    Your task is to create a clear, concise, and informative commit message based on 
    the provided information. Follow the conventional commit format if applicable.";

    let context_prompt = format!(
        "Project root: {}\n
Current branch: {}\n
Recent commits:\n{}\n
Staged changes:\n{}\n
Unstaged files:\n{}",
        git_info.project_root,
        git_info.branch,
        git_info.recent_commits.join("\n"),
        format_staged_files(&git_info.staged_files),
        git_info.unstaged_files.join("\n")
    );

    let full_prompt = format!(
        "{}\n\nContext:\n{}\n\nBased on this information, generate an appropriate commit message:",
        base_prompt, context_prompt
    );

    Ok(full_prompt)
}

fn format_staged_files(staged_files: &std::collections::HashMap<String, FileChange>) -> String {
    staged_files
        .iter()
        .map(|(file, change)| format!(
            "File: {} ({})\nDiff:\n{}\n",
            file,
            format_file_status(&change.status),
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

fn truncate_diff(diff: &str, max_length: usize) -> String {
    if diff.len() > max_length {
        format!("{}... (truncated)", &diff[..max_length])
    } else {
        diff.to_string()
    }
}