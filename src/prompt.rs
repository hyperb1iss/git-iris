use anyhow::Result;
use crate::git::{GitInfo, FileChange};
use crate::config::Config;

pub fn create_prompt(git_info: &GitInfo, config: &Config) -> Result<String> {
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