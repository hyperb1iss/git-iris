use anyhow::Result;
use crate::git::GitInfo;

pub fn create_prompt(initial_message: &str, git_info: &GitInfo) -> Result<String> {
    let base_prompt = "You are an AI assistant helping to refine Git commit messages. 
    Your task is to create a clear, concise, and informative commit message based on 
    the provided information. Follow the conventional commit format if applicable.";

    let context_prompt = format!(
        "Current branch: {}\n\nRecent commits:\n{}\n\nStaged changes:\n{}",
        git_info.branch,
        git_info.recent_commits.join("\n"),
        format_staged_files(&git_info.staged_files)
    );

    let full_prompt = format!(
        "{}\n\nContext:\n{}\n\nInitial commit message: {}\n\nRefined commit message:",
        base_prompt, context_prompt, initial_message
    );

    Ok(full_prompt)
}

fn format_staged_files(staged_files: &std::collections::HashMap<String, String>) -> String {
    staged_files
        .iter()
        .map(|(file, diff)| format!("File: {}\nDiff:\n{}\n", file, diff))
        .collect::<Vec<String>>()
        .join("\n")
}