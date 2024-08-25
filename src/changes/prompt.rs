use super::{
    change_analyzer::{calculate_total_metrics, AnalyzedChange},
    models::{ChangelogResponse, ReleaseNotesResponse},
};
use crate::common::{get_combined_instructions, DetailLevel};
use crate::config::Config;
use crate::gitmoji::get_gitmoji_list;

pub fn create_changelog_system_prompt(config: &Config) -> String {
    let changelog_schema = schemars::schema_for!(ChangelogResponse);
    let changelog_schema_str = serde_json::to_string_pretty(&changelog_schema).unwrap();

    let mut prompt = String::from(
        "You are an AI assistant specialized in generating clear, concise, and informative changelogs for software projects. \
        Your task is to create a well-structured changelog based on the provided commit information and analysis. \
        The changelog should adhere to the Keep a Changelog 1.1.0 format (https://keepachangelog.com/en/1.1.0/).

        Work step-by-step and follow these guidelines exactly:

        1. Categorize changes into the following types: Added, Changed, Deprecated, Removed, Fixed, Security.
        2. Use present tense and imperative mood in change descriptions.
        3. Start each change entry with a capital letter and do not end with a period.
        4. Be concise but descriptive in change entries and ensure good grammar, capitalization, and punctuation.
        5. Include *short* commit hashes at the end of each entry.
        6. Focus on the impact and significance of the changes and omit trivial changes below the relevance threshold.
        7. Find commonalities and group related changes together under the appropriate category.
        8. List the most impactful changes first within each category.
        9. Mention associated issue numbers and pull request numbers when available.
        10. Clearly identify and explain any breaking changes.
        11. Avoid common cliché words (like 'enhance', 'streamline', 'leverage', etc) and phrases.
        12. Do not speculate about the purpose of a change or add any information not directly supported by the context.
        13. Mention any changes to dependencies or build configurations under the appropriate category.
        14. Highlight changes that affect multiple parts of the codebase or have cross-cutting concerns.
        15. NO YAPPING!

        Your response must be a valid JSON object with the following structure:

        {
          \"version\": \"string or null\",
          \"release_date\": \"string or null\",
          \"sections\": {
            \"Added\": [{ \"description\": \"string\", \"commit_hashes\": [\"string\"], \"associated_issues\": [\"string\"], \"pull_request\": \"string or null\" }],
            \"Changed\": [...],
            \"Deprecated\": [...],
            \"Removed\": [...],
            \"Fixed\": [...],
            \"Security\": [...]
          },
          \"breaking_changes\": [{ \"description\": \"string\", \"commit_hash\": \"string\" }],
          \"metrics\": {
            \"total_commits\": number,
            \"files_changed\": number,
            \"insertions\": number,
            \"deletions\": number
          }
        }

        Follow these steps to generate the changelog:

        1. Analyze the provided commit information and group changes by type (Added, Changed, etc.).
        2. For each change type, create an array of change entries with description, commit hashes, associated issues, and pull request (if available).
        3. Identify any breaking changes and add them to the breaking_changes array.
        4. Calculate the metrics based on the overall changes.
        5. If provided, include the version and release date.
        6. Construct the final JSON object ensuring all required fields are present.

        Here's a minimal example of the expected output format:

        {
          \"version\": \"1.0.0\",
          \"release_date\": \"2023-08-15\",
          \"sections\": {
            \"Added\": [
              {
                \"description\": \"add new feature X\",
                \"commit_hashes\": [\"abc123\"],
              }
            ],
            \"Changed\": [],
            \"Deprecated\": [],
            \"Removed\": [],
            \"Fixed\": [],
            \"Security\": []
          },
          \"breaking_changes\": [],
          \"metrics\": {
            \"total_commits\": 1,
            \"files_changed\": 3,
            \"insertions\": 100,
            \"deletions\": 50
          }
        }

        Ensure that your response is a valid JSON object matching this structure. Include all required fields, even if they are empty arrays or null values.
        "
    );

    prompt.push_str(&changelog_schema_str);

    prompt.push_str(get_combined_instructions(config).as_str());

    if config.use_gitmoji {
        prompt.push_str(
            "\n\nWhen generating the changelog, include tasteful, appropriate, and intelligent use of emojis to add visual interest.\n \
            Here are some examples of emojis you can use:\n");
        prompt.push_str(&get_gitmoji_list());
    }

    prompt.push_str(
        "\n\nYou will be provided with detailed information about each change, including file-level analysis, impact scores, and classifications. \
        Use this information to create a comprehensive and insightful changelog. \
        Adjust the level of detail based on the specified detail level (Minimal, Standard, or Detailed)."
    );

    prompt
}

pub fn create_release_notes_system_prompt(config: &Config) -> String {
    let release_notes_schema = schemars::schema_for!(ReleaseNotesResponse);
    let release_notes_schema_str = serde_json::to_string_pretty(&release_notes_schema).unwrap();

    let mut prompt = String::from(
        "You are an AI assistant specialized in generating comprehensive and user-friendly release notes for software projects. \
        Your task is to create detailed release notes based on the provided commit information and analysis. \
        Aim for a tone that is professional, approachable, and authoritative, keeping in mind any additional user instructions.

        Work step-by-step and follow these guidelines exactly:

        1. Provide a high-level summary of the release, highlighting key features, improvements, and fixes.
        2. Find commonalities and group changes into meaningful sections (e.g., 'New Features', 'Improvements', 'Bug Fixes', 'Breaking Changes').
        3. Focus on the impact and benefits of the changes to users and developers.
        4. Highlight any significant new features or major improvements.
        5. Explain the rationale behind important changes when possible.
        6. Note any breaking changes and provide clear upgrade instructions.
        7. Mention any changes to dependencies or system requirements.
        8. Include any relevant documentation updates or new resources for users.
        9. Use clear, non-technical language where possible to make the notes accessible to a wide audience.
        10. Provide context for technical changes when necessary.
        11. Highlight any security updates or important bug fixes.
        12. Include overall metrics to give context about the scope of the release.
        13. Mention associated issue numbers and pull request numbers when relevant.
        14. NO YAPPING!

        Your response must be a valid JSON object with the following structure:

        {
          \"version\": \"string or null\",
          \"release_date\": \"string or null\",
          \"sections\": {
            \"Added\": [{ \"description\": \"string\", \"commit_hashes\": [\"string\"], \"associated_issues\": [\"string\"], \"pull_request\": \"string or null\" }],
            \"Changed\": [...],
            \"Deprecated\": [...],
            \"Removed\": [...],
            \"Fixed\": [...],
            \"Security\": [...]
          },
          \"breaking_changes\": [{ \"description\": \"string\", \"commit_hash\": \"string\" }],
          \"metrics\": {
            \"total_commits\": number,
            \"files_changed\": number,
            \"insertions\": number,
            \"deletions\": number
            \"total_lines_changed\": number
          }
        }

        Follow these steps to generate the changelog:

        1. Analyze the provided commit information and group changes by type (Added, Changed, etc.).
        2. For each change type, create an array of change entries with description, commit hashes, associated issues, and pull request (if available).
        3. Identify any breaking changes and add them to the breaking_changes array.
        4. Calculate the metrics based on the overall changes.
        5. If provided, include the version and release date.
        6. Construct the final JSON object ensuring all required fields are present.

        Here's a minimal example of the expected output format:

        {
          \"version\": \"1.0.0\",
          \"release_date\": \"2023-08-15\",
          \"sections\": {
            \"Added\": [
              {
                \"description\": \"add new feature X\",
                \"commit_hashes\": [\"abc123\"],
                \"associated_issues\": [\"#42\"],
                \"pull_request\": \"PR #100\"
              }
            ],
            \"Changed\": [],
            \"Deprecated\": [],
            \"Removed\": [],
            \"Fixed\": [],
            \"Security\": []
          },
          \"breaking_changes\": [],
          \"metrics\": {
            \"total_commits\": 1,
            \"files_changed\": 3,
            \"insertions\": 100,
            \"deletions\": 50
            \"total_lines_changed\": 150
          }
        }

        Ensure that your response is a valid JSON object matching this structure. Include all required fields, even if they are empty arrays or null values.
        "
    );

    prompt.push_str(&release_notes_schema_str);

    prompt.push_str(get_combined_instructions(config).as_str());

    if config.use_gitmoji {
        prompt.push_str(
            "\n\nWhen generating the release notes, include tasteful, appropriate, and intelligent use of emojis to add visual interest.\n \
            Here are some examples of emojis you can use:\n");
        prompt.push_str(&get_gitmoji_list());
    }

    prompt
}

pub fn create_changelog_user_prompt(
    changes: &[AnalyzedChange],
    detail_level: DetailLevel,
    from: &str,
    to: &str,
    readme_summary: Option<&str>,
) -> String {
    let mut prompt = format!(
        "Based on the following changes from {} to {}, generate a changelog:\n\n",
        from, to
    );

    let total_metrics = calculate_total_metrics(changes);
    prompt.push_str("Overall Changes:\n");
    prompt.push_str(&format!("Total commits: {}\n", changes.len()));
    prompt.push_str(&format!("Files changed: {}\n", total_metrics.files_changed));
    prompt.push_str(&format!(
        "Total lines changed: {}\n",
        total_metrics.total_lines_changed
    ));
    prompt.push_str(&format!("Insertions: {}\n", total_metrics.insertions));
    prompt.push_str(&format!("Deletions: {}\n\n", total_metrics.deletions));

    for change in changes {
        prompt.push_str(&format!("Commit: {}\n", change.commit_hash));
        prompt.push_str(&format!("Author: {}\n", change.author));
        prompt.push_str(&format!("Message: {}\n", change.commit_message));
        prompt.push_str(&format!("Type: {:?}\n", change.change_type));
        prompt.push_str(&format!("Breaking Change: {}\n", change.is_breaking_change));
        prompt.push_str(&format!(
            "Associated Issues: {}\n",
            change.associated_issues.join(", ")
        ));
        if let Some(pr) = &change.pull_request {
            prompt.push_str(&format!("Pull Request: {}\n", pr));
        }
        prompt.push_str(&format!(
            "Files changed: {}\n",
            change.metrics.files_changed
        ));
        prompt.push_str(&format!(
            "Lines changed: {}\n",
            change.metrics.total_lines_changed
        ));
        prompt.push_str(&format!("Insertions: {}\n", change.metrics.insertions));
        prompt.push_str(&format!("Deletions: {}\n", change.metrics.deletions));
        prompt.push_str(&format!("Impact score: {:.2}\n", change.impact_score));

        match detail_level {
            DetailLevel::Minimal => {
                // For minimal detail, we don't include file-level changes
            }
            DetailLevel::Standard | DetailLevel::Detailed => {
                prompt.push_str("File changes:\n");
                for file_change in &change.file_changes {
                    prompt.push_str(&format!(
                        "  - {} ({:?})\n",
                        file_change.new_path, file_change.change_type
                    ));
                    if detail_level == DetailLevel::Detailed {
                        for analysis in &file_change.analysis {
                            prompt.push_str(&format!("    * {}\n", analysis));
                        }
                    }
                }
            }
        }

        prompt.push('\n');
    }

    if let Some(summary) = readme_summary {
        prompt.push_str("\nProject README Summary:\n");
        prompt.push_str(summary);
        prompt.push_str("\n\n");
    }

    prompt.push_str(&format!("Please generate a {} changelog for the changes from {} to {}, adhering to the Keep a Changelog format. ", 
        match detail_level {
            DetailLevel::Minimal => "concise",
            DetailLevel::Standard => "comprehensive",
            DetailLevel::Detailed => "highly detailed",
        },
        from,
        to
    ));

    prompt.push_str("Categorize the changes appropriately and focus on the most significant updates and their impact on the project. ");
    prompt.push_str("For each change, provide a clear description of what was changed, adhering to the guidelines in the system prompt. ");
    prompt.push_str("Include the commit hashes, associated issues, and pull request numbers for each entry when available. ");
    prompt.push_str("Clearly identify and explain any breaking changes. ");

    if readme_summary.is_some() {
        prompt.push_str("Use the README summary to provide context about the project and ensure the changelog reflects the project's goals and main features. ");
    }

    prompt
}

pub fn create_release_notes_user_prompt(
    changes: &[AnalyzedChange],
    detail_level: DetailLevel,
    from: &str,
    to: &str,
    readme_summary: Option<&str>,
) -> String {
    let mut prompt = format!(
        "Based on the following changes from {} to {}, generate release notes:\n\n",
        from, to
    );

    let total_metrics = calculate_total_metrics(changes);
    prompt.push_str("Overall Changes:\n");
    prompt.push_str(&format!("Total commits: {}\n", changes.len()));
    prompt.push_str(&format!("Files changed: {}\n", total_metrics.files_changed));
    prompt.push_str(&format!(
        "Total lines changed: {}\n",
        total_metrics.total_lines_changed
    ));
    prompt.push_str(&format!("Insertions: {}\n", total_metrics.insertions));
    prompt.push_str(&format!("Deletions: {}\n\n", total_metrics.deletions));

    for change in changes {
        prompt.push_str(&format!("Commit: {}\n", change.commit_hash));
        prompt.push_str(&format!("Author: {}\n", change.author));
        prompt.push_str(&format!("Message: {}\n", change.commit_message));
        prompt.push_str(&format!("Type: {:?}\n", change.change_type));
        prompt.push_str(&format!("Breaking Change: {}\n", change.is_breaking_change));
        prompt.push_str(&format!(
            "Associated Issues: {}\n",
            change.associated_issues.join(", ")
        ));
        if let Some(pr) = &change.pull_request {
            prompt.push_str(&format!("Pull Request: {}\n", pr));
        }
        prompt.push_str(&format!("Impact score: {:.2}\n", change.impact_score));

        match detail_level {
            DetailLevel::Minimal => {
                // For minimal detail, we don't include file-level changes
            }
            DetailLevel::Standard | DetailLevel::Detailed => {
                prompt.push_str("File changes:\n");
                for file_change in &change.file_changes {
                    prompt.push_str(&format!(
                        "  - {} ({:?})\n",
                        file_change.new_path, file_change.change_type
                    ));
                    if detail_level == DetailLevel::Detailed {
                        for analysis in &file_change.analysis {
                            prompt.push_str(&format!("    * {}\n", analysis));
                        }
                    }
                }
            }
        }

        prompt.push('\n');
    }

    if let Some(summary) = readme_summary {
        prompt.push_str("\nProject README Summary:\n");
        prompt.push_str(summary);
        prompt.push_str("\n\n");
    }

    prompt.push_str(&format!(
        "Please generate {} release notes for the changes from {} to {}. ",
        match detail_level {
            DetailLevel::Minimal => "concise",
            DetailLevel::Standard => "comprehensive",
            DetailLevel::Detailed => "highly detailed",
        },
        from,
        to
    ));

    prompt.push_str("Focus on the impact and benefits of the changes to users and developers. ");
    prompt.push_str("Highlight key features, improvements, and fixes. ");
    prompt.push_str("Include a high-level summary of the release, major changes, and any breaking changes or important upgrade notes. ");
    prompt.push_str("Group changes into meaningful sections and explain the rationale behind important changes when possible. ");
    prompt.push_str("Include associated issue numbers and pull request numbers when relevant. ");

    match detail_level {
        DetailLevel::Minimal => {
            prompt.push_str(
                "Keep the release notes brief and focused on the most significant changes. ",
            );
        }
        DetailLevel::Standard => {
            prompt.push_str("Provide a balanced overview of all important changes, with some details on major features or fixes. ");
        }
        DetailLevel::Detailed => {
            prompt.push_str("Include detailed explanations of changes, their rationale, and potential impact on the project or workflow. ");
            prompt.push_str("Provide context for technical changes and include file-level details where relevant. ");
        }
    }

    if readme_summary.is_some() {
        prompt.push_str("Ensure the release notes align with the project's overall goals and main features as described in the README summary. ");
    }

    prompt.push_str(
        "Incorporate the overall metrics to give context about the scope of this release. ",
    );
    prompt.push_str("Pay special attention to changes with high impact scores, as they are likely to be the most significant. ");

    prompt
}
