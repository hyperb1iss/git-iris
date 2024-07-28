use git_iris::prompt::create_prompt;
use git_iris::git::{GitInfo, FileChange};
use git_iris::config::Config;
use std::collections::HashMap;

fn create_mock_git_info() -> GitInfo {
    GitInfo {
        branch: "feature/new-feature".to_string(),
        recent_commits: vec![
            "abc1234 Add user authentication".to_string(),
            "def5678 Update README.md".to_string(),
        ],
        staged_files: {
            let mut map = HashMap::new();
            map.insert(
                "src/main.rs".to_string(),
                FileChange {
                    status: "M".to_string(),
                    diff: "--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,5 +1,6 @@\n use std::io;\n+use std::fs;\n \n fn main() {\n     println!(\"Hello, world!\");\n".to_string(),
                },
            );
            map
        },
        unstaged_files: vec!["README.md".to_string()],
        project_root: "/home/user/projects/my-project".to_string(),
    }
}

fn create_mock_config() -> Config {
    Config {
        api_key: "dummy_api_key".to_string(),
        use_gitmoji: false,
        custom_instructions: "Always include a brief summary of changes.".to_string(),
    }
}

#[test]
fn test_create_prompt_basic() {
    let git_info = create_mock_git_info();
    let config = create_mock_config();
    let verbose = false;

    let prompt = create_prompt(&git_info, &config, verbose).unwrap();

    assert!(prompt.contains("Branch: feature/new-feature"));
    assert!(prompt.contains("abc1234 Add user authentication"));
    assert!(prompt.contains("def5678 Update README.md"));
    assert!(prompt.contains("src/main.rs (Modified, Rust source file)"));
    assert!(prompt.contains("README.md"));
    assert!(prompt.contains("Always include a brief summary of changes."));
}

#[test]
fn test_create_prompt_with_gitmoji() {
    let git_info = create_mock_git_info();
    let mut config = create_mock_config();
    config.use_gitmoji = true;
    let verbose = false;

    let prompt = create_prompt(&git_info, &config, verbose).unwrap();

    assert!(prompt.contains("Use a single gitmoji at the start of the commit message"));
}

#[test]
fn test_create_prompt_verbose() {
    let git_info = create_mock_git_info();
    let config = create_mock_config();
    let verbose = true;

    let prompt = create_prompt(&git_info, &config, verbose).unwrap();

    assert!(prompt.contains("use std::fs;"));
}

#[test]
fn test_create_prompt_with_multiple_files() {
    let mut git_info = create_mock_git_info();
    git_info.staged_files.insert(
        "src/lib.rs".to_string(),
        FileChange {
            status: "A".to_string(),
            diff: "--- /dev/null\n+++ b/src/lib.rs\n@@ -0,0 +1,3 @@\n+pub fn add(a: i32, b: i32) -> i32 {\n+    a + b\n+}\n".to_string(),
        },
    );

    let config = create_mock_config();
    let verbose = false;

    let prompt = create_prompt(&git_info, &config, verbose).unwrap();

    assert!(prompt.contains("src/main.rs (Modified, Rust source file)"));
    assert!(prompt.contains("src/lib.rs (Added, Rust source file)"));
}

#[test]
fn test_create_prompt_with_custom_instructions() {
    let git_info = create_mock_git_info();
    let mut config = create_mock_config();
    config.custom_instructions = "Use imperative mood. Mention ticket numbers if applicable.".to_string();
    let verbose = false;

    let prompt = create_prompt(&git_info, &config, verbose).unwrap();

    assert!(prompt.contains("Use imperative mood. Mention ticket numbers if applicable."));
}