use crate::config::Config;
use crate::context::CommitContext;

use anyhow::Result;

pub fn create_changelog_system_prompt(config: &Config) -> String {
    let use_emoji = config.use_gitmoji;
    let instructions = &config.instructions;

    let mut prompt = String::from(
        "You are an AI assistant specialized in generating clear, concise, and informative changelogs for software projects. \
        Your task is to create a well-structured changelog based solely on the provided commit information. \
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
        11. Do not include any wrap-up or conclusion in the changelog.
        12. NO YAPPING!"
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

    prompt.push_str(
        "\n\nUse the following output format for the changelog (you may embellish as necessary based on the instructions):
            ## Changelog

            ### New Features
            - **[Feature Title]**
            - **Description**: [Detailed description of the new feature, explaining its purpose and impact.]
            - **Commit**: [Commit Hash] ([Author])
            - **Files Affected**: [List of affected files]
            - **Detailed Changes**:
                - [Description of the specific changes made, including any new files added, modifications, etc.]
                - [Any relevant code snippets or diffs]

            ### Bug Fixes
            - **[Bug Fix Title]**
            - **Description**: [Detailed description of the bug fix, explaining the issue and how it was resolved.]
            - **Commit**: [Commit Hash] ([Author])
            - **Files Affected**: [List of affected files]
            - **Detailed Changes**:
                - [Description of the specific changes made to fix the bug, including any relevant code snippets or diffs]
                - [Any tests added or modified]

            ### Performance Improvements
            - **[Improvement Title]**
            - **Description**: [Detailed description of the performance improvement, explaining the issue and how performance was enhanced.]
            - **Commit**: [Commit Hash] ([Author])
            - **Files Affected**: [List of affected files]
            - **Detailed Changes**:
                - [Description of the specific changes made to improve performance, including any relevant code snippets or diffs]

            ### Refactoring
            - **[Refactor Title]**
            - **Description**: [Detailed description of the refactoring effort, explaining the purpose and impact of the refactor.]
            - **Commit**: [Commit Hash] ([Author])
            - **Files Affected**: [List of affected files]
            - **Detailed Changes**:
                - [Description of the specific changes made during the refactoring, including any relevant code snippets or diffs]

            ### Breaking Changes
            - **[Breaking Change Title]**
            - **Description**: [Detailed description of the breaking change, explaining why it is considered breaking and what users need to know.]
            - **Commit**: [Commit Hash] ([Author])
            - **Files Affected**: [List of affected files]
            - **Detailed Changes**:
                - [Description of the specific changes that are breaking, including any relevant code snippets or diffs]
            - **Migration Steps**: [Steps users need to follow to adapt to the breaking change, if applicable]

            ### Other Changes
            - **[Other Change Title]**
            - **Description**: [Detailed description of any other changes that do not fit into the above categories.]
            - **Commit**: [Commit Hash] ([Author])
            - **Files Affected**: [List of affected files]
            - **Detailed Changes**:
                - [Description of the specific changes made, including any relevant code snippets or diffs]

            ## Additional Notes
            - **Documentation Updates**: [Description of any updates made to the documentation.]
            - **Deprecations**: [List of any deprecated features or functionality, along with alternative recommendations.]
            - **Known Issues**: [List of any known issues that have not been resolved in this release.]"
    );
    prompt
}

pub fn create_changelog_user_prompt(commits: Vec<CommitContext>) -> String {
    "Create a well-structured changelog based on the provided commit information. Group changes by type and include the commit hash for each entry.\n\n".to_string()
        + &format_commits(commits)
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
        4. Include a brief conclusion or forward-looking statement.
        5. Ensure the release notes are informative, well-structured, and suitable for both technical and non-technical readers.
        6. Focus on the impact and benefits of the changes rather than implementation details.
        7. Avoid common cliché words (like 'enhance', 'streamline', 'leverage', etc) and phrases.
        8. Do not speculate about the purpose of a change or add any information not directly supported by the context.
        9. If there's not enough information to create a complete, authoritative entry, state only what can be confidently determined from the context.
        10. Do not include any wrap-up or conclusion in the release notes.
        11. NO YAPPING!"
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

    prompt.push_str(
        "\n\nUse the following output format for the release notes (you may embellish as necessary based on the instructions):
            ## Version [Version Number]
            - **Release Date**: [Date]

            ### Summary
            [Provide a high-level summary of the release, highlighting key features, improvements, and fixes.]

            ### Major Changes

            #### New Features
            - **[Feature Title]**
            - **Description**: [Detailed description of the new feature, explaining its purpose and impact.]
            - **Benefit**: [Explain the benefits or advantages of this feature to the users.]

            #### Improvements
            - **[Improvement Title]**
            - **Description**: [Detailed description of the improvement, explaining how it enhances the application.]
            - **Benefit**: [Explain the benefits or advantages of this improvement to the users.]

            #### Bug Fixes
            - **[Bug Fix Title]**
            - **Description**: [Detailed description of the bug fix, explaining the issue and how it was resolved.]
            - **Impact**: [Explain the impact of the bug fix on the users.]

            ### Breaking Changes
            - **[Breaking Change Title]**
            - **Description**: [Detailed description of the breaking change, explaining why it is considered breaking and what users need to know.]
            - **Impact**: [Explain the impact of the breaking change on the users.]
            - **Migration Steps**: [Steps users need to follow to adapt to the breaking change.]

            ### Additional Notes
            - **Documentation Updates**: [Description of any updates made to the documentation.]
            - **Deprecations**: [List of any deprecated features or functionality, along with alternative recommendations.]
            - **Known Issues**: [List of any known issues that have not been resolved in this release.]"
    );

    prompt
}

pub fn create_release_notes_user_prompt(changelog: String) -> String {
    "Create detailed release notes based on the provided changelog. Include a high-level summary of the release, major changes, and any breaking changes or important upgrade notes.\n\n".to_string()
        + &changelog.clone()
}

pub fn create_breaking_changes_prompt(
    commits: Vec<CommitContext>,
    config: &Config,
) -> Result<String> {
    let system_prompt =
        create_breaking_changes_system_prompt(config.use_gitmoji, &config.instructions);
    let user_prompt = create_breaking_changes_user_prompt(commits);

    let full_prompt = format!("{}\n\n{}", system_prompt, user_prompt);

    Ok(full_prompt)
}

fn create_breaking_changes_system_prompt(use_emoji: bool, instructions: &str) -> String {
    let mut prompt = String::from(
        "You are an AI assistant specialized in identifying and explaining breaking changes in software updates. \
        Your task is to analyze the provided changelog and identify any breaking changes. \
        Aim for a tone that is professional, approachable, and authoritative, keeping in mind any additional user instructions.

        Work step-by-step and follow these guidelines exactly:

        1. List any changes that might require users to modify their code or update their workflows.
        2. Provide clear explanations of each breaking change.
        3. If possible, suggest migration steps or workarounds for each breaking change.
        4. If no breaking changes are found, explicitly state that there are no breaking changes in this release.
        5. Consider API changes, dependency updates, removed features, or significant behavior changes as potential breaking changes.
        6. Avoid common cliché words (like 'enhance', 'streamline', 'leverage', etc) and phrases.
        7. Do not speculate about the purpose of a change or add any information not directly supported by the context.
        8. If there's not enough information to create a complete, authoritative entry, state only what can be confidently determined from the context."
    );

    if use_emoji {
        prompt.push_str(
            "\n\nWhen generating the list of breaking changes, include tasteful, appropriate, and intelligent use of emojis to add visual interest.\n\n",
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

fn create_breaking_changes_user_prompt(commits: Vec<CommitContext>) -> String {
    "Analyze the provided changelog and identify any breaking changes. 
    Provide clear explanations and suggest migration steps or workarounds where possible.\n\n"
        .to_string()
        + &format_commits(commits)
}

fn format_commits(commits: Vec<CommitContext>) -> String {
    let mut commit_contexts = String::new();

    for commit in &commits {
        let commit_context = format!(
            "Commit: {}\nAuthor: {}\nDate: {}\nMessage: {}\n\n",
            commit.recent_commits[0].hash,
            commit.recent_commits[0].author,
            commit.recent_commits[0].timestamp,
            commit.recent_commits[0].message
        );
        commit_contexts.push_str(&commit_context);
    }
    let first_commit = &commits[0].recent_commits[0];
    let last_commit = &commits[commits.len() - 1].recent_commits[0];

    format!(
        "First commit: {}\nLast commit: {}\n\n",
        first_commit.hash, last_commit.hash
    ) + &commit_contexts
}
