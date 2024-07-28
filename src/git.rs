use anyhow::{Result, anyhow};
use std::process::Command;
use std::collections::HashMap;

#[derive(Debug)]
pub struct GitInfo {
    pub branch: String,
    pub recent_commits: Vec<String>,
    pub staged_files: HashMap<String, String>,
}

pub fn get_git_info() -> Result<GitInfo> {
    let branch = get_current_branch()?;
    let recent_commits = get_recent_commits(5)?;
    let staged_files = get_staged_files_with_diff()?;

    Ok(GitInfo {
        branch,
        recent_commits,
        staged_files,
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

fn get_staged_files_with_diff() -> Result<HashMap<String, String>> {
    let staged_files_output = Command::new("git")
        .args(&["diff", "--name-only", "--cached"])
        .output()?;

    let staged_files: Vec<String> = String::from_utf8(staged_files_output.stdout)?
        .lines()
        .map(|s| s.to_string())
        .collect();

    let mut staged_files_with_diff = HashMap::new();

    for file in staged_files {
        let diff_output = Command::new("git")
            .args(&["diff", "--cached", &file])
            .output()?;

        let diff = String::from_utf8(diff_output.stdout)?;
        staged_files_with_diff.insert(file, diff);
    }

    Ok(staged_files_with_diff)
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