use git_iris::config::Config;
use git_iris::git::{FileChange, GitInfo};
use git_iris::prompt::{create_prompt, create_user_prompt};
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

    let prompt = create_prompt(&git_info, &config, false).unwrap();

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

    let prompt = create_prompt(&git_info, &config, false).unwrap();

    assert!(prompt.contains("Branch: main"));
    assert!(prompt.contains("file1.rs (Modified"));
    assert!(prompt.contains("- old line\n+ new line"));
}

#[test]
fn test_create_prompt_with_gitmoji() {
    let git_info = create_mock_git_info();
    let mut config = Config::default();
    config.use_gitmoji = true;

    let prompt = create_prompt(&git_info, &config, false).unwrap();

    println!("{}", prompt);
    assert!(prompt.contains("Use a single gitmoji at the start of the commit message"));
    assert!(prompt.contains("🎨 - :art: - Improve structure / format of the code"));
}

#[test]
fn test_create_prompt_with_custom_instructions() {
    let git_info = create_mock_git_info();
    let mut config = Config::default();
    config.custom_instructions = "Always mention the ticket number".to_string();

    let prompt = create_prompt(&git_info, &config, false).unwrap();

    assert!(prompt.contains("Always mention the ticket number"));
}

#[test]
fn test_create_prompt_verbose() {
    let git_info = create_mock_git_info();
    let config = Config::default();

    let prompt = create_prompt(&git_info, &config, true).unwrap();

    assert!(prompt.contains("Detailed changes"));
}

#[test]
fn test_create_user_prompt() {
    let git_info = create_mock_git_info();
    let prompt = create_user_prompt(&git_info, false).unwrap();

    assert!(prompt.contains("Branch: main"));
    assert!(prompt.contains("abcdef1 Initial commit"));
    assert!(prompt.contains("file1.rs (Modified"));
    assert!(prompt.contains("- old line\n+ new line"));
    assert!(prompt.contains("unstaged_file.txt"));
}
