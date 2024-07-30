use crate::config::Config;
use crate::context::{ChangeType, CommitContext, ProjectMetadata, RecentCommit, StagedFile};
use crate::gitmoji::{apply_gitmoji, get_gitmoji_list};
use crate::log_debug;
use crate::relevance::RelevanceScorer;
use anyhow::Result;
use std::collections::HashMap;

pub fn create_prompt(context: &CommitContext, config: &Config, verbose: bool) -> Result<String> {
    let system_prompt = create_system_prompt(config.use_gitmoji, &config.custom_instructions);
    let user_prompt = create_user_prompt(context, verbose)?;

    let full_prompt = format!("{}\n\n{}", system_prompt, user_prompt);

    if verbose {
        log_debug!("Full Prompt:\n{}", full_prompt);
    }

    Ok(full_prompt)
}

pub fn create_system_prompt(use_gitmoji: bool, custom_instructions: &str) -> String {
    let mut prompt = String::from(
        "You are an AI assistant specializing in creating high-quality, professional Git commit messages. \
        Your task is to generate clear, concise, and informative commit messages based on the provided context. \
        Aim for a tone that is professional yet approachable, keeping in mind any additional user instructions.

        Work step-by-step and follow these guidelines exactly:
        
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
        12. Do not include a conclusion or end summary section.
        13. Keep the message concise and to the point, avoiding unnecessary elaboration.
        14. Avoid common cliché words (like 'enhance', 'delve', etc) and phrases.
        15. Don't mention filenames in the subject line unless absolutely necessary.
        16. NO YAPPING!

        Remember, a good commit message should complete the following sentence:
        If applied, this commit will... <your subject line here>
        (but don't actually state this in your response).

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
            "\n\nAdditional user-supplied instructions:\n{}\n\n",
            custom_instructions
        ));
    }

    prompt
}

pub fn create_user_prompt(context: &CommitContext, verbose: bool) -> Result<String> {
    let scorer = RelevanceScorer::new();
    let relevance_scores = scorer.score(context);
    let detailed_changes = format_detailed_changes(&context.staged_files, &relevance_scores);

    let prompt = format!(
        "Based on the following context, generate a Git commit message:\n\n\
        Branch: {}\n\n\
        Recent commits:\n{}\n\n\
        Staged changes:\n{}\n\n\
        Unstaged files:\n{}\n\n\
        Project metadata:\n{}\n\n\
        Detailed changes:\n{}",
        context.branch,
        format_recent_commits(&context.recent_commits),
        format_staged_files(&context.staged_files, &relevance_scores),
        context.unstaged_files.join(", "),
        format_project_metadata(&context.project_metadata),
        detailed_changes
    );

    if verbose {
        log_debug!("Detailed changes:\n{}", detailed_changes);
        log_debug!("User Prompt:\n{}", prompt);
    }

    Ok(prompt)
}

fn format_recent_commits(commits: &[RecentCommit]) -> String {
    commits
        .iter()
        .map(|commit| format!("{} - {}", &commit.hash[..7], commit.message))
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_staged_files(files: &[StagedFile], relevance_scores: &HashMap<String, f32>) -> String {
    files
        .iter()
        .map(|file| {
            let relevance = relevance_scores.get(&file.path).unwrap_or(&0.0);
            format!(
                "{} ({:.2}) - {}",
                file.path,
                relevance,
                format_change_type(&file.change_type)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_project_metadata(metadata: &ProjectMetadata) -> String {
    format!(
        "Language: {}\nFramework: {}\nDependencies: {}",
        metadata.language,
        metadata.framework.as_deref().unwrap_or("None"),
        metadata.dependencies.join(", ")
    )
}

fn format_detailed_changes(
    files: &[StagedFile],
    relevance_scores: &HashMap<String, f32>,
) -> String {
    files
        .iter()
        .map(|file| {
            let relevance = relevance_scores.get(&file.path).unwrap_or(&0.0);
            format!(
                "File: {} (Relevance: {:.2})\nChange Type: {}\nAnalysis:\n{}\n\nDiff:\n{}",
                file.path,
                relevance,
                format_change_type(&file.change_type),
                file.analysis.join("\n"),
                file.diff
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n---\n\n")
}

fn format_change_type(change_type: &ChangeType) -> &'static str {
    match change_type {
        ChangeType::Added => "Added",
        ChangeType::Modified => "Modified",
        ChangeType::Deleted => "Deleted",
    }
}

pub fn process_commit_message(message: String, use_gitmoji: bool) -> String {
    if use_gitmoji {
        apply_gitmoji(&message)
    } else {
        message
    }
}
