use crate::config::Config;
use crate::git::{FileChange, GitInfo};
use crate::gitmoji::{apply_gitmoji, get_gitmoji_list};
use crate::log_debug;
use anyhow::Result;

pub fn create_system_prompt(use_gitmoji: bool, custom_instructions: &str) -> String {
    let mut prompt = String::from(
        "You are an AI assistant specializing in creating high-quality, professional Git commit messages. \
        Your task is to generate clear, concise, and informative commit messages based on the provided context. \
        Aim for a tone that is professional yet approachable. Follow these guidelines:
        
        1. Use the imperative mood in the subject line (e.g., 'Add feature' not 'Added feature').
        2. Limit the subject line to 50 characters if possible, but never exceed 72 characters.
        3. Capitalize the subject line.
        4. Do not end the subject line with a period.
        5. Separate subject from body with a blank line.
        6. Wrap the body at 72 characters.
        7. Use the body to explain what changes you made and why, not how.
        8. Be specific and avoid vague commit messages.
        9. Focus on the impact and purpose of the changes, not just what files were modified.
        10. If the changes are part of a larger feature or fix, provide that context.
        11. For non-trivial changes, include a brief explanation of the motivation behind the change.
        12. Do not include a conclusion or end summary section in the commit message.
        13. Keep the message concise and to the point, avoiding unnecessary elaboration.

        Remember, a good commit message should complete the following sentence:
        If applied, this commit will... <your subject line here>

        Generate only the commit message, without any explanations or additional text."
    );

    if use_gitmoji {
        prompt.push_str(
            "\n\nUse a single gitmoji at the start of the commit message. \
            Choose the most relevant emoji from the following list:\n\n",
        );
        prompt.push_str(&get_gitmoji_list());
    }

    if !custom_instructions.is_empty() {
        prompt.push_str(&format!(
            "\n\nAdditional user-supplied instructions:\n{}",
            custom_instructions
        ));
    }

    prompt
}

pub fn create_user_prompt(
    git_info: &GitInfo,
    verbose: bool,
    existing_message: Option<&str>,
) -> Result<String> {
    let mut prompt = format!(
        "Based on the following context, generate a Git commit message:\n\n\
        Branch: {}\n\n\
        Recent commits:\n{}\n\n\
        Staged changes:\n{}\n\n\
        Unstaged files:\n{}\n\n\
        Detailed changes:\n{}",
        git_info.branch,
        format_recent_commits(&git_info.recent_commits),
        format_staged_files(&git_info.staged_files),
        git_info.unstaged_files.join(", "),
        format_detailed_changes(&git_info.staged_files, &git_info.project_root)?
    );

    if let Some(message) = existing_message {
        prompt.push_str(&format!("\n\nExisting commit message:\n{}\n", message));
        prompt.push_str("\nPlease refine the existing commit message based on the following instructions:\n");
    } else {
        prompt.push_str("\n\nAdditional context provided by the user:\n");
    }

    if verbose {
        log_debug!("User Prompt:\n{}", prompt);
    }

    Ok(prompt)
}

pub fn create_prompt(
    git_info: &GitInfo,
    config: &Config,
    verbose: bool,
    existing_message: Option<&str>,
) -> Result<String> {
    let system_prompt = create_system_prompt(config.use_gitmoji, &config.custom_instructions);
    let user_prompt = create_user_prompt(git_info, verbose, existing_message)?;

    let full_prompt = format!("{}\n\n{}", system_prompt, user_prompt);

    if verbose {
        log_debug!("Full Prompt:\n{}", full_prompt);
    }

    Ok(full_prompt)
}

fn format_recent_commits(commits: &[String]) -> String {
    commits.join("\n")
}

fn format_staged_files(staged_files: &std::collections::HashMap<String, FileChange>) -> String {
    staged_files
        .iter()
        .map(|(file, change)| format!("{} ({})", file, format_file_status(&change.status),))
        .collect::<Vec<String>>()
        .join("\n")
}

fn format_detailed_changes(
    staged_files: &std::collections::HashMap<String, FileChange>,
    project_root: &str,
) -> Result<String> {
    let mut detailed_changes = Vec::new();

    for (file, change) in staged_files {
        let relative_path = file.strip_prefix(project_root).unwrap_or(file);
        let file_type = "Unknown"; // We can't determine the file type without accessing the file system
        let file_analysis: Vec<String> = Vec::new(); // We can't analyze the file without accessing the file system

        detailed_changes.push(format!(
            "File: {} ({}, {})\n\nAnalysis:\n{}\n\nDiff:\n{}",
            relative_path,
            format_file_status(&change.status),
            file_type,
            if file_analysis.is_empty() {
                "No significant patterns detected.".to_string()
            } else {
                file_analysis.join(", ")
            },
            change.diff,
        ));
    }

    Ok(detailed_changes.join("\n\n---\n\n"))
}

fn format_file_status(status: &str) -> &str {
    match status {
        "A" => "Added",
        "M" => "Modified",
        "D" => "Deleted",
        _ => "Changed",
    }
}

pub fn process_commit_message(message: String, use_gitmoji: bool) -> String {
    if use_gitmoji {
        apply_gitmoji(&message)
    } else {
        message
    }
}
