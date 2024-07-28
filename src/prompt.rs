use crate::config::Config;
use crate::file_analyzers::{self};
use crate::git::{FileChange, GitInfo};
use anyhow::Result;
use regex::Regex;
use std::fs;
use std::process::Command;

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
        8. If applicable, use conventional commit types (e.g., feat:, fix:, docs:, style:, refactor:, test:, chore:).
        9. When multiple files or changes are involved, summarize the overall change in the subject line and use bullet points in the body for details.
        10. Be specific and avoid vague commit messages.
        11. Focus on the impact and purpose of the changes, not just what files were modified.
        12. If the changes are part of a larger feature or fix, provide that context.
        13. For non-trivial changes, include a brief explanation of the motivation behind the change.
        14. Do not include a conclusion or end summary section in the commit message.
        15. Keep the message concise and to the point, avoiding unnecessary elaboration.
        16. Do not include any section labels (e.g., 'Commit Message:', 'Commit Body:') in your response.
        17. Format your response as follows: subject line, blank line, then the commit body.

        Remember, a good commit message should complete the following sentence:
        If applied, this commit will... <your subject line here>

        Generate only the commit message, without any explanations or additional text."
    );

    if use_gitmoji {
        prompt.push_str(
            "\n\n18. Use a single gitmoji at the start of the commit message. \
            Choose the most relevant emoji. Some common gitmoji include:
            🎨 - :art: - Improve structure / format of the code.
            ⚡️ - :zap: - Improve performance.
            🔥 - :fire: - Remove code or files.
            🐛 - :bug: - Fix a bug.
            🚑️ - :ambulance: - Critical hotfix.
            ✨ - :sparkles: - Introduce new features.
            📝 - :memo: - Add or update documentation.
            🚀 - :rocket: - Deploy stuff.
            💄 - :lipstick: - Add or update the UI and style files.
            🎉 - :tada: - Begin a project.
            ✅ - :white_check_mark: - Add, update, or pass tests.
            🔒️ - :lock: - Fix security or privacy issues.
            🔐 - :closed_lock_with_key: - Add or update secrets.
            🔖 - :bookmark: - Release / Version tags.
            🚨 - :rotating_light: - Fix compiler / linter warnings.
            🚧 - :construction: - Work in progress.
            💚 - :green_heart: - Fix CI Build.
            ⬇️ - :arrow_down: - Downgrade dependencies.
            ⬆️ - :arrow_up: - Upgrade dependencies.
            📌 - :pushpin: - Pin dependencies to specific versions.
            👷 - :construction_worker: - Add or update CI build system.
            📈 - :chart_with_upwards_trend: - Add or update analytics or track code.
            ♻️ - :recycle: - Refactor code.
            ➕ - :heavy_plus_sign: - Add a dependency.
            ➖ - :heavy_minus_sign: - Remove a dependency.
            🔧 - :wrench: - Add or update configuration files.
            🔨 - :hammer: - Add or update development scripts.
            🌐 - :globe_with_meridians: - Internationalization and localization.
            ✏️ - :pencil2: - Fix typos.
            💩 - :poop: - Write bad code that needs to be improved.
            ⏪️ - :rewind: - Revert changes.
            🔀 - :twisted_rightwards_arrows: - Merge branches.
            📦️ - :package: - Add or update compiled files or packages.
            👽️ - :alien: - Update code due to external API changes.
            🚚 - :truck: - Move or rename resources (e.g.: files, paths, routes).
            📄 - :page_facing_up: - Add or update license.
            💥 - :boom: - Introduce breaking changes.
            🍱 - :bento: - Add or update assets.
            ♿️ - :wheelchair: - Improve accessibility.
            💡 - :bulb: - Add or update comments in source code.
            🍻 - :beers: - Write code drunkenly.
            💬 - :speech_balloon: - Add or update text and literals.
            🗃️ - :card_file_box: - Perform database related changes.
            🔊 - :loud_sound: - Add or update logs.
            🔇 - :mute: - Remove logs.
            👥 - :busts_in_silhouette: - Add or update contributor(s).
            🚸 - :children_crossing: - Improve user experience / usability.
            🏗️ - :building_construction: - Make architectural changes.
            📱 - :iphone: - Work on responsive design.
            🤡 - :clown_face: - Mock things.
            🥚 - :egg: - Add or update an easter egg.
            🙈 - :see_no_evil: - Add or update a .gitignore file.
            📸 - :camera_flash: - Add or update snapshots.
            ⚗️ - :alembic: - Perform experiments.
            🔍️ - :mag: - Improve SEO.
            🏷️ - :label: - Add or update types.
            🌱 - :seedling: - Add or update seed files.
            🚩 - :triangular_flag_on_post: - Add, update, or remove feature flags.
            🥅 - :goal_net: - Catch errors.
            💫 - :dizzy: - Add or update animations and transitions.
            🗑️ - :wastebasket: - Deprecate code that needs to be cleaned up.
            🛂 - :passport_control: - Work on code related to authorization, roles and permissions.
            🩹 - :adhesive_bandage: - Simple fix for a non-critical issue.
            🧐 - :monocle_face: - Data exploration/inspection.
            ⚰️ - :coffin: - Remove dead code.
            🧪 - :test_tube: - Add a failing test.
            👔 - :necktie: - Add or update business logic.
            🩺 - :stethoscope: - Add or update healthcheck.
            🧱 - :bricks: - Infrastructure related changes.
            🧑‍💻 - :technologist: - Improve developer experience.
            💸 - :money_with_wings: - Add sponsorships or money related infrastructure.
            🧵 - :thread: - Add or update code related to multithreading or concurrency.
            🦺 - :safety_vest: - Add or update code related to validation.",
        );
    }

    if !custom_instructions.is_empty() {
        prompt.push_str("\n\nAdditional instructions:\n");
        for instruction in custom_instructions.split('\n') {
            prompt.push_str(&format!("- {}\n", instruction.trim()));
        }
    }

    prompt
}

pub fn create_user_prompt(git_info: &GitInfo, verbose: bool) -> Result<String> {
    let context = format!(
        "Branch: {}\n\nRecent commits:\n{}\n\nStaged changes:\n{}\n\nUnstaged files:\n{}\n\nDetailed changes:\n{}",
        git_info.branch,
        format_recent_commits(&git_info.recent_commits),
        format_staged_files(&git_info.staged_files),
        git_info.unstaged_files.join(", "),
        format_detailed_changes(&git_info.staged_files, &git_info.project_root)?
    );

    let prompt = format!(
        "Based on the following context, generate a Git commit message:\n\n{}",
        context
    );

    if verbose {
        println!("User Prompt:\n{}", prompt);
    }

    Ok(prompt)
}

pub fn create_prompt(git_info: &GitInfo, config: &Config, verbose: bool) -> Result<String> {
    let system_prompt = create_system_prompt(config.use_gitmoji, &config.custom_instructions);
    let user_prompt = create_user_prompt(git_info, verbose)?;

    let full_prompt = format!("{}\n\n{}", system_prompt, user_prompt);

    if verbose {
        println!("Full Prompt:\n{}", full_prompt);
    }

    Ok(full_prompt)
}
fn format_recent_commits(commits: &[String]) -> String {
    commits
        .iter()
        .map(|commit| {
            if let Some(ticket) = extract_ticket_number(commit) {
                format!("{} ({})", commit, ticket)
            } else {
                commit.to_string()
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn format_staged_files(staged_files: &std::collections::HashMap<String, FileChange>) -> String {
    staged_files
        .iter()
        .map(|(file, change)| {
            let analyzer = file_analyzers::get_analyzer(file);
            format!(
                "{} ({}, {})",
                file,
                format_file_status(&change.status),
                analyzer.get_file_type()
            )
        })
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
        let analyzer = file_analyzers::get_analyzer(file);
        let file_type = analyzer.get_file_type();
        let file_analysis = analyzer.analyze(file, change);

        let (file_content_before, file_content_after) =
            if change.status != "D" && change.status != "A" {
                (get_file_content_before(file)?, fs::read_to_string(file)?)
            } else if change.status == "A" {
                (String::new(), fs::read_to_string(file)?)
            } else {
                (get_file_content_before(file)?, String::new())
            };

        detailed_changes.push(format!(
            "File: {} ({}, {})\n\nAnalysis:\n{}\n\nDiff:\n{}\n\nFile content before changes:\n{}\n\nFile content after changes:\n{}",
            relative_path,
            format_file_status(&change.status),
            file_type,
            if file_analysis.is_empty() { "No significant patterns detected.".to_string() } else { file_analysis.join(", ") },
            change.diff,
            if change.status == "A" { "New file".to_string() } else { file_content_before },
            if change.status == "D" { "File deleted".to_string() } else { file_content_after }
        ));
    }

    Ok(detailed_changes.join("\n\n---\n\n"))
}

fn get_file_content_before(file: &str) -> Result<String> {
    let output = Command::new("git")
        .args(&["show", &format!("HEAD:{}", file)])
        .output()?;

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn format_file_status(status: &str) -> &str {
    match status {
        "A" => "Added",
        "M" => "Modified",
        "D" => "Deleted",
        _ => "Changed",
    }
}

fn extract_ticket_number(commit: &str) -> Option<String> {
    let re = Regex::new(r"(TICKET-\d+)").unwrap();
    re.captures(commit)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
}
