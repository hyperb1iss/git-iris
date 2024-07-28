use crate::config::Config;
use crate::file_analyzers::{self, FileAnalyzer};
use crate::git::{FileChange, GitInfo};
use anyhow::Result;
use regex::Regex;
use std::fs;
use std::process::Command;

pub fn create_prompt(git_info: &GitInfo, config: &Config, verbose: bool) -> Result<String> {
    let context = format!(
        "Branch: {}\n\nRecent commits:\n{}\n\nStaged changes:\n{}\n\nUnstaged files:\n{}\n\nDetailed changes:\n{}",
        git_info.branch,
        format_recent_commits(&git_info.recent_commits),
        format_staged_files(&git_info.staged_files),
        git_info.unstaged_files.join(", "),
        format_detailed_changes(&git_info.staged_files, &git_info.project_root)?
    );

    let prompt = format!(
        "Generate a Git commit message {} based on the following context:\n\n{}

Guidelines:
1. Use imperative mood in subject
2. 50 char subject line, 72 char body wrap
3. Explain what and why, not how
4. Focus on significant changes and their purpose
5. No backticks for filenames
6. Don't list modified functions without explaining their purpose or impact
7. Provide meaningful context for changes, not just what was changed
8. Consider the full file contents when explaining changes
9. Use bullet points for multiple changes or aspects, not 'firstly', 'secondly', etc.
10. Don't end bullet points with a period
11. Use a blank line before starting bullet points in the commit body",
        if config.use_gitmoji {
            "with appropriate gitmoji"
        } else {
            ""
        },
        context
    );

    if verbose {
        println!("Prompt:\n{}", prompt);
    }

    Ok(prompt)
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
