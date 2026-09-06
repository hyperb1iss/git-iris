use git_iris::common::CommonParams;
use git_iris::config::Config;
use git_iris::providers::ProviderConfig;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};

// Use our centralized test infrastructure
#[path = "test_utils.rs"]
mod test_utils;
use test_utils::{MockDataBuilder, setup_git_repo};

fn cwd_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().expect("lock")
}

// Helper to verify git repo status
fn is_git_repo(dir: &Path) -> bool {
    let status = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(dir)
        .output()
        .expect("Failed to execute git command");

    status.status.success()
}

#[test]
fn test_project_config_security() {
    let _guard = cwd_lock();
    // Set up a git repository using our centralized infrastructure
    let (temp_dir, _git_repo) = setup_git_repo();

    // Save current directory so we can restore it later
    let original_dir = env::current_dir().expect("Failed to get current directory");

    // Change to the test repository directory for the test
    env::set_current_dir(temp_dir.path()).expect("Failed to change to test directory");

    // Verify we're in a git repo
    assert!(
        is_git_repo(Path::new(".")),
        "Current directory is not a git repository"
    );

    // 1. Test API key security in project config
    // Create a config with API keys using our MockDataBuilder
    let mut config = MockDataBuilder::config();

    // Add API keys to multiple providers
    for provider_name in &["openai", "anthropic", "google"] {
        let provider_config = ProviderConfig {
            api_key: format!("secret_{provider_name}_api_key"),
            model: format!("{provider_name}_model"),
            ..Default::default()
        };

        config
            .providers
            .insert((*provider_name).to_string(), provider_config);
    }

    // Save as project config
    config
        .save_as_project_config()
        .expect("Failed to save project config");

    // Verify the file exists
    let config_path = Path::new(".irisconfig");
    assert!(config_path.exists(), "Project config file not created");

    // Read the file content
    let content = fs::read_to_string(config_path).expect("Failed to read project config file");

    // Verify no API keys are in the file
    for provider_name in &["openai", "anthropic", "google"] {
        let api_key = format!("secret_{provider_name}_api_key");
        assert!(
            !content.contains(&api_key),
            "API key was found in project config file"
        );
    }

    // 2. Test merging project config with personal config
    // Create configs using our MockDataBuilder
    let mut personal_config =
        MockDataBuilder::test_config_with_api_key("openai", "personal_api_key");
    personal_config
        .providers
        .get_mut("openai")
        .expect("OpenAI provider should exist")
        .model = "gpt-5.4".to_string();

    let mut project_config = MockDataBuilder::config();
    let project_provider_config = ProviderConfig {
        api_key: String::new(), // Empty API key
        model: "gpt-5.4".to_string(),
        ..Default::default()
    };
    project_config
        .providers
        .insert("openai".to_string(), project_provider_config);

    // Merge configs
    personal_config.merge_with_project_config(project_config);

    // Verify API key from personal config is preserved
    let provider_config = personal_config
        .providers
        .get("openai")
        .expect("OpenAI provider config not found");
    assert_eq!(
        provider_config.api_key, "personal_api_key",
        "Personal API key was lost during merge"
    );

    // Verify model from project config is used
    assert_eq!(
        provider_config.model, "gpt-5.4",
        "Project model setting was not applied"
    );

    // 3. Test CLI command integration
    // Set up common parameters similar to CLI arguments
    let common = CommonParams {
        provider: Some("openai".to_string()),
        model: None,
        instructions: Some("Test instructions".to_string()),
        preset: Some("default".to_string()),
        gitmoji: Some(true),
        gitmoji_flag: false,
        no_gitmoji: false,
        critic_flag: false,
        no_critic: false,
        critic: None,
        repository_url: None,
    };

    // Create a config using our MockDataBuilder and apply common parameters
    let mut config = MockDataBuilder::config();
    common
        .apply_to_config(&mut config)
        .expect("Failed to apply common params");

    // Set an API key
    let provider_config = config
        .providers
        .get_mut("openai")
        .expect("OpenAI provider config not found");
    provider_config.api_key = "cli_integration_api_key".to_string();

    // Save as project config
    config
        .save_as_project_config()
        .expect("Failed to save project config with CLI params");

    // Read the file content
    let content = fs::read_to_string(config_path).expect("Failed to read project config file");

    // Verify the API key is not in the file
    assert!(
        !content.contains("cli_integration_api_key"),
        "API key from CLI integration was found in project config file"
    );

    // Clean up - restore original directory
    env::set_current_dir(original_dir).expect("Failed to restore original directory");
}

#[test]
fn test_project_config_only_serializes_changed_values() {
    // Test Config serialization directly without file system operations
    // This avoids current directory race conditions in parallel tests
    let config = Config {
        default_provider: String::new(),
        providers: HashMap::new(),
        use_gitmoji: false, // Explicitly changed from default
        instructions: String::new(),
        instruction_preset: "conventional".to_string(), // Explicitly changed from default
        theme: String::new(),
        subagent_timeout_secs: 120,
        subagent_max_turns: 20,
        critic_enabled: true,
        temp_instructions: None,
        temp_preset: None,
        is_project_config: true,
        gitmoji_override: None,
        critic_override: None,
    };

    let content = toml::to_string_pretty(&config).expect("Failed to serialize config");

    // Should contain the explicitly set values
    assert!(
        content.contains("use_gitmoji = false"),
        "use_gitmoji should be serialized when false. Got:\n{content}"
    );
    assert!(
        content.contains(r#"instruction_preset = "conventional""#),
        "instruction_preset should be serialized when non-default. Got:\n{content}"
    );

    // Should NOT contain default/empty values
    assert!(
        !content.contains("default_provider"),
        "default_provider should NOT be serialized when empty. Got:\n{content}"
    );
    assert!(
        !content.contains("theme"),
        "theme should NOT be serialized when empty. Got:\n{content}"
    );
    assert!(
        !content.contains("subagent_timeout"),
        "subagent_timeout should NOT be serialized when default (120). Got:\n{content}"
    );
    assert!(
        !content.contains("subagent_max_turns"),
        "subagent_max_turns should NOT be serialized when default (20). Got:\n{content}"
    );
    assert!(
        !content.contains("instructions ="),
        "empty instructions should NOT be serialized. Got:\n{content}"
    );
    assert!(
        !content.contains("[providers]"),
        "empty providers table should NOT be serialized. Got:\n{content}"
    );
}

#[test]
fn test_project_config_with_provider_only_serializes_set_fields() {
    // Test Config serialization directly without file system operations
    // This avoids current directory race conditions in parallel tests
    let mut providers = HashMap::new();
    providers.insert(
        "anthropic".to_string(),
        ProviderConfig {
            api_key: String::new(),
            model: "claude-opus-4-6".to_string(),
            fast_model: None,
            subagent_model: None,
            token_limit: None,
            additional_params: HashMap::new(),
        },
    );

    let config = Config {
        default_provider: "anthropic".to_string(),
        providers,
        use_gitmoji: true, // default, should NOT serialize
        instructions: String::new(),
        instruction_preset: "default".to_string(), // default, should NOT serialize
        theme: String::new(),
        subagent_timeout_secs: 120,
        subagent_max_turns: 20,
        critic_enabled: true,
        temp_instructions: None,
        temp_preset: None,
        is_project_config: true,
        gitmoji_override: None,
        critic_override: None,
    };

    let content = toml::to_string_pretty(&config).expect("Failed to serialize config");

    // Should contain provider and model
    assert!(
        content.contains(r#"default_provider = "anthropic""#),
        "default_provider should be serialized when set. Got:\n{content}"
    );
    assert!(
        content.contains("[providers.anthropic]"),
        "provider section should exist. Got:\n{content}"
    );
    assert!(
        content.contains(r#"model = "claude-opus-4-6""#),
        "model should be serialized. Got:\n{content}"
    );

    // Should NOT contain defaults
    assert!(
        !content.contains("use_gitmoji"),
        "use_gitmoji=true (default) should NOT be serialized. Got:\n{content}"
    );
    assert!(
        !content.contains("instruction_preset"),
        "instruction_preset=default should NOT be serialized. Got:\n{content}"
    );
    assert!(
        !content.contains("theme"),
        "empty theme should NOT be serialized. Got:\n{content}"
    );
    assert!(
        !content.contains("api_key"),
        "empty api_key should NOT be serialized. Got:\n{content}"
    );
    assert!(
        !content.contains("fast_model"),
        "None fast_model should NOT be serialized. Got:\n{content}"
    );
    assert!(
        !content.contains("token_limit"),
        "None token_limit should NOT be serialized. Got:\n{content}"
    );
}

#[test]
fn test_subagent_max_turns_serializes_when_changed() {
    let config = Config {
        subagent_max_turns: 35,
        ..Config::default()
    };

    let content = toml::to_string_pretty(&config).expect("Failed to serialize config");

    assert!(
        content.contains("subagent_max_turns = 35"),
        "subagent_max_turns should serialize when non-default. Got:\n{content}"
    );
}

#[test]
fn test_critic_enabled_serializes_when_disabled() {
    let config = Config {
        critic_enabled: false,
        ..Config::default()
    };

    let content = toml::to_string_pretty(&config).expect("Failed to serialize config");

    assert!(
        content.contains("critic_enabled = false"),
        "critic_enabled should serialize when disabled. Got:\n{content}"
    );
}

#[test]
fn test_common_params_can_disable_critic() {
    let common = CommonParams {
        no_critic: true,
        ..CommonParams::default()
    };
    let mut config = Config::default();

    common
        .apply_to_config(&mut config)
        .expect("Failed to apply common params");

    assert!(!config.critic_enabled);
    assert_eq!(config.critic_override, Some(false));
}

#[test]
fn test_common_params_tracks_explicit_critic_opt_in() {
    let common = CommonParams {
        critic_flag: true,
        ..CommonParams::default()
    };
    let mut config = Config::default();

    common
        .apply_to_config(&mut config)
        .expect("Failed to apply common params");

    assert!(config.critic_enabled);
    assert_eq!(config.critic_override, Some(true));
}

#[test]
fn test_provider_config_skip_serialization() {
    // Test that ProviderConfig properly skips empty fields
    let config = ProviderConfig {
        api_key: String::new(),
        model: String::new(),
        fast_model: None,
        subagent_model: None,
        token_limit: None,
        additional_params: HashMap::new(),
    };

    let serialized = toml::to_string(&config).expect("Failed to serialize");

    // All fields are empty/None, should result in empty or minimal output
    assert!(
        !serialized.contains("api_key"),
        "empty api_key should not serialize"
    );
    assert!(
        !serialized.contains("model"),
        "empty model should not serialize"
    );
    assert!(
        !serialized.contains("fast_model"),
        "None fast_model should not serialize"
    );
    assert!(
        !serialized.contains("token_limit"),
        "None token_limit should not serialize"
    );
    assert!(
        !serialized.contains("additional_params"),
        "empty additional_params should not serialize"
    );
}

#[test]
fn test_provider_config_serializes_set_values() {
    let mut params = HashMap::new();
    params.insert("temperature".to_string(), "0.7".to_string());

    let config = ProviderConfig {
        api_key: String::new(), // Still empty, should skip
        model: "gpt-5.4".to_string(),
        fast_model: Some("gpt-5.4-mini".to_string()),
        subagent_model: None,
        token_limit: Some(4096),
        additional_params: params,
    };

    let serialized = toml::to_string(&config).expect("Failed to serialize");

    // Set values should appear
    assert!(
        serialized.contains(r#"model = "gpt-5.4""#),
        "model should serialize"
    );
    assert!(
        serialized.contains(r#"fast_model = "gpt-5.4-mini""#),
        "fast_model should serialize"
    );
    assert!(
        serialized.contains("token_limit = 4096"),
        "token_limit should serialize"
    );
    assert!(
        serialized.contains("temperature"),
        "additional_params should serialize"
    );

    // Empty api_key should NOT appear
    assert!(
        !serialized.contains("api_key"),
        "empty api_key should not serialize"
    );
}
