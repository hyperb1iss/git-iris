use crate::common::get_combined_instructions;
use crate::config::Config;
use crate::context::{
    ChangeType, CommitContext, GeneratedMessage, ProjectMetadata, RecentCommit, StagedFile,
};
use crate::gitmoji::{apply_gitmoji, get_gitmoji_list};

use super::relevance::RelevanceScorer;
use crate::log_debug;
use std::collections::HashMap;

pub fn create_system_prompt(config: &Config) -> anyhow::Result<String> {
    let commit_schema = schemars::schema_for!(GeneratedMessage);
    let commit_schema_str = serde_json::to_string_pretty(&commit_schema)?;

    let mut prompt = String::from(
        "You are an AI assistant specializing in creating high-quality, professional Git commit messages. \
        Your task is to generate clear, concise, and informative commit messages based solely on the provided context.
        
        Work step-by-step and follow these guidelines exactly:

        1. Use the imperative mood in the subject line (e.g., 'Add feature' not 'Added feature').
        2. Limit the subject line to 50 characters if possible, but never exceed 72 characters.
        3. Capitalize the subject line.
        4. Do not end the subject line with a period.
        5. Separate subject from body with a blank line.
        6. Wrap the body at 72 characters.
        7. Use the body to explain what changes were made and their impact, and how they were implemented.
        8. Be specific and avoid vague language.
        9. Focus on the concrete changes and their effects, not assumptions about intent.
        10. If the changes are part of a larger feature or fix, state this fact if evident from the context.
        11. For non-trivial changes, include a brief explanation of the change's purpose if clearly indicated in the context.
        12. Do not include a conclusion or end summary section.
        13. Avoid common cliché words (like 'enhance', 'streamline', 'leverage', etc) and phrases.
        14. Don't mention filenames in the subject line unless absolutely necessary.
        15. Only describe changes that are explicitly shown in the provided context.
        16. If the purpose or impact of a change is not clear from the context, focus on describing the change itself without inferring intent.
        17. Do not use phrases like 'seems to', 'appears to', or 'might be' - only state what is certain based on the context.
        18. If there's not enough information to create a complete, authoritative message, state only what can be confidently determined from the context.
        19. NO YAPPING!

        Be sure to quote newlines and any other control characters in your response.

        The message should be based entirely on the information provided in the context,
        without any speculation or assumptions.
      ");

    prompt.push_str(get_combined_instructions(config).as_str());

    if config.use_gitmoji {
        prompt.push_str(
            "\n\nUse a single gitmoji at the start of the commit message. \
          Choose the most relevant emoji from the following list:\n\n",
        );
        prompt.push_str(&get_gitmoji_list());
    }

    prompt.push_str("
        Your response must be a valid JSON object with the following structure:

        {
          \"emoji\": \"string or null\",
          \"title\": \"string\",
          \"message\": \"string\"
        }

        Follow these steps to generate the commit message:

        1. Analyze the provided context, including staged changes, recent commits, and project metadata.
        2. Identify the main purpose of the commit based on the changes.
        3. Create a concise and descriptive title (subject line) for the commit.
        4. If using emojis (false unless stated below), select the most appropriate one for the commit type.
        5. Write a detailed message body explaining the changes, their impact, and any other relevant information.
        6. Ensure the message adheres to the guidelines above, and follows all of the additional instructions provided.
        7. Construct the final JSON object with the emoji (if applicable), title, and message.

         Here's a minimal example of the expected output format:

        {
          \"emoji\": \"✨\",
          \"title\": \"Add user authentication feature\",
          \"message\": \"Implement user authentication using JWT tokens\\n\\n- Add login and registration endpoints\\n- Create middleware for token verification\\n- Update user model to include password hashing\\n- Add unit tests for authentication functions\"
        }

        Ensure that your response is a valid JSON object matching this structure. Include an empty string for the emoji if not using one.
        "
    );

    prompt.push_str(&commit_schema_str);

    Ok(prompt)
}

pub fn create_user_prompt(context: &CommitContext) -> String {
    let scorer = RelevanceScorer::new();
    let relevance_scores = scorer.score(context);
    let detailed_changes = format_detailed_changes(&context.staged_files, &relevance_scores);

    let prompt = format!(
        "Based on the following context, generate a Git commit message:\n\n\
        Branch: {}\n\n\
        Recent commits:\n{}\n\n\
        Staged changes:\n{}\n\n\
        Project metadata:\n{}\n\n\
        Detailed changes:\n{}",
        context.branch,
        format_recent_commits(&context.recent_commits),
        format_staged_files(&context.staged_files, &relevance_scores),
        format_project_metadata(&context.project_metadata),
        detailed_changes
    );

    log_debug!("Detailed changes:\n{}", detailed_changes);

    prompt
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
        metadata.language.as_deref().unwrap_or("None"),
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
            let mut file_info = format!(
                "File: {} (Relevance: {:.2})\nChange Type: {}\nAnalysis:\n{}\n\nDiff:\n{}",
                file.path,
                relevance,
                format_change_type(&file.change_type),
                file.analysis.join("\n"),
                file.diff
            );

            // Add full file content if available
            if let Some(content) = &file.content {
                file_info.push_str("\n\n---\n\nFull File Content:\n");
                file_info.push_str(content);
                file_info.push_str("\n\n--- End of File ---\n");
            }

            file_info
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
