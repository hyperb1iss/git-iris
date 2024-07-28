use git_iris::config::ProviderConfig; // Import ProviderConfig explicitly
use git_iris::{create_prompt, Config, FileChange, GitInfo};
use std::collections::HashMap;

fn create_dummy_git_info() -> GitInfo {
    GitInfo {
        branch: "main".to_string(),
        recent_commits: vec![
            "abc1234 Initial commit".to_string(),
            "def5678 Add feature X".to_string(),
        ],
        staged_files: {
            let mut map = HashMap::new();
            map.insert(
                "src/main.rs".to_string(),
                FileChange {
                    status: "M".to_string(),
                    diff: "- old line\n+ new line".to_string(),
                },
            );
            map
        },
        unstaged_files: vec!["README.md".to_string()],
        project_root: "/path/to/project".to_string(),
    }
}

fn create_dummy_config() -> Config {
    let mut providers = HashMap::new();
    providers.insert(
        "openai".to_string(),
        ProviderConfig {
            api_key: "dummy_api_key".to_string(),
            model: "gpt-3.5-turbo".to_string(),
            additional_params: HashMap::new(),
        },
    );

    Config {
        default_provider: "openai".to_string(),
        providers,
        use_gitmoji: false,
        custom_instructions: String::new(),
    }
}

#[test]
fn test_create_prompt() {
    let git_info = create_dummy_git_info();
    let config = create_dummy_config();

    let prompt = create_prompt(&git_info, &config, false).unwrap();

    assert!(prompt.contains("main"));
    assert!(prompt.contains("Initial commit"));
    assert!(prompt.contains("Add feature X"));
    assert!(prompt.contains("src/main.rs"));
    assert!(prompt.contains("- old line"));
    assert!(prompt.contains("+ new line"));
    assert!(prompt.contains("README.md"));
}

#[test]
fn test_create_prompt_with_gitmoji() {
    let git_info = create_dummy_git_info();
    let mut config = create_dummy_config();
    config.use_gitmoji = true;

    let prompt = create_prompt(&git_info, &config, false).unwrap();

    assert!(prompt.contains("gitmoji"));
}

#[test]
fn test_create_prompt_with_custom_instructions() {
    let git_info = create_dummy_git_info();
    let mut config = create_dummy_config();
    config.custom_instructions = "Always mention the ticket number".to_string();

    let prompt = create_prompt(&git_info, &config, false).unwrap();

    assert!(prompt.contains("Always mention the ticket number"));
}

#[test]
fn test_create_prompt_with_different_provider() {
    let git_info = create_dummy_git_info();
    let mut config = create_dummy_config();
    config.providers.insert(
        "claude".to_string(),
        ProviderConfig {
            api_key: "dummy_claude_api_key".to_string(),
            model: "claude-v1".to_string(),
            additional_params: HashMap::new(),
        },
    );
    config.default_provider = "claude".to_string();

    let prompt = create_prompt(&git_info, &config, false).unwrap();

    // The prompt content should be the same regardless of the provider
    assert!(prompt.contains("main"));
    assert!(prompt.contains("Initial commit"));
    assert!(prompt.contains("Add feature X"));
    assert!(prompt.contains("src/main.rs"));
    assert!(prompt.contains("- old line"));
    assert!(prompt.contains("+ new line"));
    assert!(prompt.contains("README.md"));
}
