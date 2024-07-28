use anyhow::Result;
use git_iris::config::Config;
use git_iris::git::{FileChange, GitInfo};
use git_iris::prompt::{
    create_prompt, create_system_prompt, create_user_prompt, process_commit_message,
};
use std::collections::HashMap;

fn create_mock_git_info() -> GitInfo {
    GitInfo {
        branch: "main".to_string(),
        recent_commits: vec!["abcdef1 Initial commit".to_string()],
        staged_files: {
            let mut map = HashMap::new();
            map.insert(
                "file1.rs".to_string(),
                FileChange {
                    status: "M".to_string(),
                    diff: "- old line\n+ new line".to_string(),
                },
            );
            map
        },
        unstaged_files: vec!["unstaged_file.txt".to_string()],
        project_root: "/mock/path/to/project".to_string(),
    }
}

#[test]
fn test_create_prompt_basic() {
    let git_info = create_mock_git_info();
    let config = Config::default();

    let prompt = create_prompt(&git_info, &config, false, &[], None).unwrap();

    assert!(prompt.contains("Branch: main"));
    assert!(prompt.contains("abcdef1 Initial commit"));
    assert!(prompt.contains("file1.rs (Modified"));
    assert!(prompt.contains("- old line\n+ new line"));
    assert!(prompt.contains("unstaged_file.txt"));
}

#[test]
fn test_create_prompt_with_staged_files() {
    let git_info = create_mock_git_info();
    let config = Config::default();

    let prompt = create_prompt(&git_info, &config, false, &[], None).unwrap();

    assert!(prompt.contains("Branch: main"));
    assert!(prompt.contains("file1.rs (Modified"));
    assert!(prompt.contains("- old line\n+ new line"));
}

#[test]
fn test_create_prompt_with_gitmoji() {
    let git_info = create_mock_git_info();
    let mut config = Config::default();
    config.use_gitmoji = true;

    let prompt = create_prompt(&git_info, &config, false, &[], None).unwrap();

    println!("{}", prompt);
    assert!(prompt.contains("Use a single gitmoji at the start of the commit message"));
    assert!(prompt.contains("🎨 - :art: - Improve structure / format of the code"));
}

#[test]
fn test_create_prompt_with_custom_instructions() {
    let git_info = create_mock_git_info();
    let mut config = Config::default();
    config.custom_instructions = "Always mention the ticket number".to_string();

    let prompt = create_prompt(&git_info, &config, false, &[], None).unwrap();

    assert!(prompt.contains("Always mention the ticket number"));
}

#[test]
fn test_create_prompt_with_inpaint_context() {
    let git_info = create_mock_git_info();
    let config = Config::default();

    let inpaint_context = vec![
        "This commit fixes a critical bug".to_string(),
        "The bug was causing performance issues".to_string(),
    ];

    let prompt = create_prompt(&git_info, &config, false, &inpaint_context, None).unwrap();

    assert!(prompt.contains("Branch: main"));
    assert!(prompt.contains("abcdef1 Initial commit"));
    assert!(prompt.contains("Additional context provided by the user:"));
    assert!(prompt.contains("This commit fixes a critical bug"));
    assert!(prompt.contains("The bug was causing performance issues"));
}

#[test]
fn test_create_prompt_verbose() {
    let git_info = create_mock_git_info();
    let config = Config::default();

    let prompt = create_prompt(&git_info, &config, true, &[], None).unwrap();

    assert!(prompt.contains("Detailed changes:"));
    assert!(prompt.contains("File: file1.rs (Modified"));
    assert!(prompt.contains("Diff:\n- old line\n+ new line"));
}

#[test]
fn test_create_prompt_with_multiple_commits() {
    let mut git_info = create_mock_git_info();
    git_info.recent_commits = vec![
        "abcdef1 Initial commit".to_string(),
        "123456 Add new feature".to_string(),
        "789012 Fix bug".to_string(),
    ];

    let config = Config::default();

    let prompt = create_prompt(&git_info, &config, false, &[], None).unwrap();

    assert!(prompt.contains("abcdef1 Initial commit"));
    assert!(prompt.contains("123456 Add new feature"));
    assert!(prompt.contains("789012 Fix bug"));
}

#[test]
fn test_create_system_prompt() {
    let prompt = create_system_prompt(false, "");
    assert!(prompt.contains("You are an AI assistant"));
    assert!(!prompt.contains("Use a single gitmoji"));

    let prompt_with_gitmoji = create_system_prompt(true, "");
    assert!(prompt_with_gitmoji.contains("Use a single gitmoji"));
    assert!(prompt_with_gitmoji.contains("🎨 - :art: - Improve structure / format of the code"));

    let prompt_with_custom = create_system_prompt(false, "Always mention the ticket number");
    assert!(prompt_with_custom.contains("Always mention the ticket number"));
}

#[test]
fn test_create_user_prompt() -> Result<()> {
    let git_info = GitInfo {
        branch: "main".to_string(),
        recent_commits: vec!["abc123 Initial commit".to_string()],
        staged_files: {
            let mut map = HashMap::new();
            map.insert(
                "file1.rs".to_string(),
                FileChange {
                    status: "M".to_string(),
                    diff: "- old\n+ new".to_string(),
                },
            );
            map
        },
        unstaged_files: vec!["file2.rs".to_string()],
        project_root: "/project".to_string(),
    };

    let prompt = create_user_prompt(&git_info, false, &[], None)?;
    assert!(prompt.contains("Branch: main"));
    assert!(prompt.contains("abc123 Initial commit"));
    assert!(prompt.contains("file1.rs (Modified)"));
    assert!(prompt.contains("file2.rs"));

    Ok(())
}

#[test]
fn test_process_commit_message() {
    assert_eq!(
        process_commit_message("feat: new feature".to_string(), true),
        "✨ feat: new feature"
    );
    assert_eq!(
        process_commit_message("feat: new feature".to_string(), false),
        "feat: new feature"
    );
}
