use crate::change_analyzer::AnalyzedChange;
use crate::changelog::DetailLevel;
use crate::config::Config;

pub fn create_changelog_system_prompt(config: &Config) -> String {
    let use_emoji = config.use_gitmoji;
    let instructions = &config.instructions;

    let mut prompt = String::from(
        "You are an AI assistant specialized in generating clear, concise, and informative changelogs for software projects. \
        Your task is to create a well-structured changelog based on the provided commit information and analysis. \
        Aim for a tone that is professional, approachable, and authoritative, keeping in mind any additional user instructions.

        Work step-by-step and follow these guidelines exactly:

        1. Focus on the impact and significance of the changes in addition to technical details.
        2. Use the present tense and imperative mood.
        3. Group changes by type (e.g., 'Features', 'Bug Fixes', 'Performance Improvements', 'Refactoring').
        4. For each entry, include the commit hash at the end in parentheses.
        5. Ensure the changelog is well-structured and easy to read.
        6. If a change is particularly significant or breaking, make a note of it.
        7. Be as detailed as possible about each change without speculating.
        8. Avoid common cliché words (like 'enhance', 'streamline', 'leverage', etc) and phrases.
        9. Do not speculate about the purpose of a change or add any information not directly supported by the context.
        10. If there's not enough information to create a complete, authoritative entry, state only what can be confidently determined from the context.
        11. Use the provided impact scores to prioritize and emphasize more significant changes.
        12. Incorporate file-level analysis to provide more context about the nature of the changes.
        13. Consider the number of files changed, insertions, and deletions when describing the scope of a change.
        14. Mention any changes to project dependencies or build configurations.
        15. Highlight changes that affect multiple parts of the codebase or have cross-cutting concerns.
        16. Never include a conclusion or final summary statement.
        16. NO YAPPING!"
    );

    if use_emoji {
        prompt.push_str(
            "\n\nWhen generating the changelog, include tasteful, appropriate, and intelligent use of emojis to add visual interest.\n\n",
        );
    }

    if !instructions.is_empty() {
        prompt.push_str(&format!(
            "\n\nAdditional instructions:\n{}\n\n",
            instructions
        ));
    }

    prompt.push_str(
        "\n\nYou will be provided with detailed information about each change, including file-level analysis and impact scores. \
        Use this information to create a comprehensive and insightful changelog. \
        Adjust the level of detail based on the specified detail level (Minimal, Standard, or Detailed)."
    );

    prompt
}

pub fn create_changelog_user_prompt(
    changes: &[AnalyzedChange],
    detail_level: DetailLevel,
    from: &str,
    to: &str,
) -> String {
    let mut prompt = String::from(format!(
        "Based on the following changes from {} to {}, generate a changelog:\n\n",
        from, to
    ));

    for change in changes {
        prompt.push_str(&format!("Commit: {}\n", change.commit_hash));
        prompt.push_str(&format!("Author: {}\n", change.author));
        prompt.push_str(&format!("Message: {}\n", change.commit_message));
        prompt.push_str(&format!("Files changed: {}\n", change.files_changed));
        prompt.push_str(&format!("Insertions: {}\n", change.insertions));
        prompt.push_str(&format!("Deletions: {}\n", change.deletions));
        prompt.push_str(&format!("Impact score: {:.2}\n", change.impact_score));

        match detail_level {
            DetailLevel::Minimal => {
                // For minimal detail, we don't include file-level changes
            }
            DetailLevel::Standard => {
                prompt.push_str("File changes summary:\n");
                for file_change in &change.file_changes {
                    prompt.push_str(&format!(
                        "  - {} ({})\n",
                        file_change.new_path, file_change.change_type
                    ));
                }
            }
            DetailLevel::Detailed => {
                prompt.push_str("Detailed file changes:\n");
                for file_change in &change.file_changes {
                    prompt.push_str(&format!(
                        "  - {} ({})\n",
                        file_change.new_path, file_change.change_type
                    ));
                    for analysis in &file_change.analysis {
                        prompt.push_str(&format!("    * {}\n", analysis));
                    }
                }
            }
        }

        prompt.push_str("\n");
    }

    prompt.push_str(&format!("\nPlease generate a {} changelog for the changes from {} to {}, focusing on the most significant updates and their impact on the project. ", 
        match detail_level {
            DetailLevel::Minimal => "concise",
            DetailLevel::Standard => "comprehensive",
            DetailLevel::Detailed => "highly detailed",
        },
        from,
        to
    ));

    prompt.push_str("Group the changes by type and order them by significance. ");
    prompt.push_str("For each change, provide a clear description of what was changed and, where possible, why it matters to users or developers.");

    prompt
}

pub fn create_release_notes_system_prompt(config: &Config) -> String {
    let use_emoji = config.use_gitmoji;
    let instructions = &config.instructions;

    let mut prompt = String::from(
        "You are an AI assistant specialized in generating comprehensive and user-friendly release notes for software projects. \
        Your task is to create detailed release notes based on the provided changelog. \
        Aim for a tone that is professional, approachable, and authoritative, keeping in mind any additional user instructions.

        Work step-by-step and follow these guidelines exactly:

        1. Provide a high-level summary of the release, highlighting key features, improvements, and fixes.
        2. Include a bulleted list of major changes, grouped by type (e.g., 'New Features', 'Improvements', 'Bug Fixes').
        3. Note any breaking changes or important upgrade notes.
        4. Never include a conclusion or final summary statement.
        5. Ensure the release notes are informative, well-structured, and suitable for both technical and non-technical readers.
        6. Focus on the impact and benefits of the changes rather than implementation details.
        7. Avoid common cliché words (like 'enhance', 'streamline', 'leverage', etc) and phrases.
        8. Do not speculate about the purpose of a change or add any information not directly supported by the context.
        9. If there's not enough information to create a complete, authoritative entry, state only what can be confidently determined from the context.
        10. Highlight any significant performance improvements or optimizations.
        11. Mention any changes to dependencies or system requirements.
        12. Include any relevant documentation updates or new resources for users.
        13. NO YAPPING!"
    );

    if use_emoji {
        prompt.push_str(
            "\n\nWhen generating the release notes, include tasteful, appropriate, and intelligent use of emojis to add visual interest.\n\n",
        );
    }

    if !instructions.is_empty() {
        prompt.push_str(&format!(
            "\n\nAdditional instructions:\n{}\n\n",
            instructions
        ));
    }

    prompt
}

pub fn create_release_notes_user_prompt(
    changelog: &str,
    detail_level: DetailLevel,
    from: &str,
    to: &str,
) -> String {
    let mut prompt = String::from(format!(
        "Based on the following changelog for changes from {} to {}, generate release notes:\n\n",
        from, to
    ));
    prompt.push_str(changelog);

    prompt.push_str(&format!("\n\nPlease generate {} release notes for the changes from {} to {} based on this changelog. ", 
        match detail_level {
            DetailLevel::Minimal => "concise",
            DetailLevel::Standard => "comprehensive",
            DetailLevel::Detailed => "highly detailed",
        },
        from,
        to
    ));

    prompt.push_str("Include a high-level summary of the release, major changes, and any breaking changes or important upgrade notes. ");
    prompt.push_str("Focus on the impact and benefits of the changes to users and developers. ");

    match detail_level {
        DetailLevel::Minimal => {
            prompt.push_str(
                "Keep the release notes brief and focused on the most significant changes.",
            );
        }
        DetailLevel::Standard => {
            prompt.push_str("Provide a balanced overview of all important changes, with some details on major features or fixes.");
        }
        DetailLevel::Detailed => {
            prompt.push_str("Include detailed explanations of changes, their rationale, and potential impact on the project or workflow.");
        }
    }

    prompt
}
