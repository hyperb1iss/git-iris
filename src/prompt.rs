use anyhow::Result;
use crate::git::{GitInfo, FileChange};
use crate::config::Config;
use std::path::Path;
use regex::Regex;
use crate::file_analyzers::{self, FileAnalyzer};

pub fn create_prompt(git_info: &GitInfo, config: &Config, verbose: bool) -> Result<String> {
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
Unstaged files:\n{}\n
Change analysis:\n{}",
        git_info.project_root,
        git_info.branch,
        format_recent_commits(&git_info.recent_commits),
        format_staged_files(&git_info.staged_files),
        git_info.unstaged_files.join("\n"),
        analyze_changes(&git_info.staged_files, &git_info.project_root)
    );

    let full_prompt = format!(
        "{}\n\nContext:\n{}\n\nBased on this information, generate an appropriate commit message. Focus on the overall purpose of the changes, and include specific details only if they are significant. Do not use backticks around filenames, and avoid mentioning minor changes like individual import additions.",
        base_prompt, context_prompt
    );

    if verbose {
        println!("Prompt being sent to LLM:\n{}", full_prompt);
    }

    Ok(full_prompt)
}

fn format_recent_commits(commits: &[String]) -> String {
    commits.iter()
        .map(|commit| {
            if let Some(ticket) = extract_ticket_number(commit) {
                format!("{} (Ticket: {})", commit, ticket)
            } else {
                commit.to_string()
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn extract_ticket_number(commit: &str) -> Option<String> {
    let re = Regex::new(r"(TICKET-\d+)").unwrap();
    re.captures(commit)
        .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()))
}

fn format_staged_files(staged_files: &std::collections::HashMap<String, FileChange>) -> String {
    staged_files
        .iter()
        .map(|(file, change)| {
            let analyzer = file_analyzers::get_analyzer(file);
            format!(
                "File: {} ({}) - {}\nDiff:\n{}\n",
                file,
                format_file_status(&change.status),
                analyzer.get_file_type(),
                truncate_diff(&change.diff, 500)
            )
        })
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

fn analyze_changes(staged_files: &std::collections::HashMap<String, FileChange>, project_root: &str) -> String {
    let mut analysis = Vec::new();

    for (file, change) in staged_files {
        let relative_path = file.strip_prefix(project_root).unwrap_or(file);
        let analyzer = file_analyzers::get_analyzer(file);
        
        let file_analysis = analyzer.analyze(file, change);
        if !file_analysis.is_empty() {
            analysis.push(format!("{}:\n  {}", relative_path, file_analysis.join("\n  ")));
        }
    }

    if analysis.is_empty() {
        "No significant patterns detected in the changes.".to_string()
    } else {
        analysis.join("\n")
    }
}