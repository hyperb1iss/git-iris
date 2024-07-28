use anyhow::{Result, anyhow};
use std::process::Command;
use std::collections::HashMap;

#[derive(Debug)]
pub struct GitInfo {
    pub branch: String,
    pub recent_commits: Vec<String>,
    pub staged_files: HashMap<String, FileChange>,
    pub unstaged_files: Vec<String>,
    pub project_root: String,
}

#[derive(Debug)]
pub struct FileChange {
    pub status: String,
    pub diff: String,
}

pub fn get_git_info() -> Result<GitInfo> {
    let branch = get_current_branch()?;
    let recent_commits = get_recent_commits(5)?;
    let staged_files = get_staged_files_with_diff()?;
    let unstaged_files = get_unstaged_files()?;
    let project_root = get_project_root()?;

    Ok(GitInfo {
        branch,
        recent_commits,
        staged_files,
        unstaged_files,
        project_root,
    })
}

fn get_current_branch() -> Result<String> {
    let output = Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .output()?;

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn get_recent_commits(count: usize) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(&["log", &format!("-{}", count), "--oneline"])
        .output()?;

    Ok(String::from_utf8(output.stdout)?
        .lines()
        .map(|s| s.to_string())
        .collect())
}

fn get_staged_files_with_diff() -> Result<HashMap<String, FileChange>> {
    let status_output = Command::new("git")
        .args(&["status", "--porcelain"])
        .output()?;

    let status_lines = String::from_utf8(status_output.stdout)?;
    let mut staged_files_with_diff = HashMap::new();

    for line in status_lines.lines() {
        let status = &line[0..2];
        let file = &line[3..];

        if status.starts_with('A') || status.starts_with('M') || status.starts_with('D') {
            let diff_output = Command::new("git")
                .args(&["diff", "--cached", file])
                .output()?;

            let diff = String::from_utf8(diff_output.stdout)?;
            staged_files_with_diff.insert(file.to_string(), FileChange {
                status: status.to_string(),
                diff,
            });
        }
    }

    Ok(staged_files_with_diff)
}

fn get_unstaged_files() -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(&["ls-files", "--others", "--exclude-standard"])
        .output()?;

    Ok(String::from_utf8(output.stdout)?
        .lines()
        .map(|s| s.to_string())
        .collect())
}

fn get_project_root() -> Result<String> {
    let output = Command::new("git")
        .args(&["rev-parse", "--show-toplevel"])
        .output()?;

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

pub fn commit(message: &str) -> Result<()> {
    let status = Command::new("git")
        .args(&["commit", "-m", message])
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("Failed to commit changes"))
    }
}