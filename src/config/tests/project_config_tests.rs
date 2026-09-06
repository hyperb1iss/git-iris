use crate::config::Config;

#[test]
fn explicit_project_defaults_override_personal_config() {
    let mut personal_config = Config {
        default_provider: "anthropic".to_string(),
        use_gitmoji: false,
        instructions: "personal instructions".to_string(),
        instruction_preset: "conventional".to_string(),
        theme: "midnight".to_string(),
        subagent_timeout_secs: 300,
        subagent_max_turns: 40,
        ..Config::default()
    };

    let project_toml = r#"
default_provider = "openai"
use_gitmoji = true
instructions = ""
instruction_preset = "default"
theme = ""
subagent_timeout_secs = 120
subagent_max_turns = 20
critic_enabled = true
"#;
    let mut project_config: Config = toml::from_str(project_toml).expect("valid project config");
    project_config.is_project_config = true;
    let project_source: toml::Value =
        toml::from_str(project_toml).expect("valid project config source");

    personal_config.merge_loaded_project_config(project_config, &project_source);

    assert_eq!(personal_config.default_provider, "openai");
    assert!(personal_config.use_gitmoji);
    assert_eq!(personal_config.instructions, "");
    assert_eq!(personal_config.instruction_preset, "default");
    assert_eq!(personal_config.theme, "");
    assert_eq!(personal_config.subagent_timeout_secs, 120);
    assert_eq!(personal_config.subagent_max_turns, 20);
    assert!(personal_config.critic_enabled);
}

#[test]
fn explicit_project_subagent_max_turns_overrides_personal_config() {
    let mut personal_config = Config {
        subagent_max_turns: 40,
        ..Config::default()
    };

    let project_toml = "subagent_max_turns = 12";
    let mut project_config: Config = toml::from_str(project_toml).expect("valid project config");
    project_config.is_project_config = true;
    let project_source: toml::Value =
        toml::from_str(project_toml).expect("valid project config source");

    personal_config.merge_loaded_project_config(project_config, &project_source);

    assert_eq!(personal_config.subagent_max_turns, 12);
}

#[test]
fn explicit_project_critic_setting_overrides_personal_config() {
    let mut personal_config = Config {
        critic_enabled: true,
        ..Config::default()
    };

    let project_toml = "critic_enabled = false";
    let mut project_config: Config = toml::from_str(project_toml).expect("valid project config");
    project_config.is_project_config = true;
    let project_source: toml::Value =
        toml::from_str(project_toml).expect("valid project config source");

    personal_config.merge_loaded_project_config(project_config, &project_source);

    assert!(!personal_config.critic_enabled);
}

#[test]
fn migrate_legacy_gemini_provider_to_google() {
    let config_toml = r#"
default_provider = "gemini"

[providers.gemini]
api_key = "AIza-example-key"
model = "gemini-3-pro-preview"
fast_model = "gemini-2.5-flash"
"#;

    let config: Config = toml::from_str(config_toml).expect("valid config");
    let (migrated, needs_save) = Config::migrate_if_needed(config);

    assert!(
        needs_save,
        "legacy gemini provider should trigger migration"
    );
    assert_eq!(migrated.default_provider, "google");
    assert!(!migrated.providers.contains_key("gemini"));

    let google_config = migrated
        .providers
        .get("google")
        .expect("google config should exist after migration");
    assert_eq!(google_config.model, "gemini-3-pro-preview");
    assert_eq!(
        google_config.fast_model.as_deref(),
        Some("gemini-2.5-flash")
    );
    assert!(migrated.validate().is_ok());
}

#[test]
fn canonical_provider_config_wins_over_legacy_alias() {
    let config_toml = r#"
default_provider = "gemini"

[providers.google]
api_key = "AIza-canonical"
model = "gemini-3-pro-preview"

[providers.gemini]
api_key = "AIza-legacy"
model = "gemini-2.5-flash"
"#;

    let config: Config = toml::from_str(config_toml).expect("valid config");
    let (migrated, needs_save) = Config::migrate_if_needed(config);

    assert!(needs_save, "legacy alias should trigger migration");
    assert_eq!(migrated.default_provider, "google");
    assert!(!migrated.providers.contains_key("gemini"));

    let google_config = migrated
        .providers
        .get("google")
        .expect("google config should exist after migration");
    assert_eq!(google_config.api_key, "AIza-canonical");
    assert_eq!(google_config.model, "gemini-3-pro-preview");
}

/// Regression: `migrate_if_needed` must never touch the filesystem. A prior
/// version called `config.save()` internally, which would stomp the user's
/// live `~/.config/git-iris/config.toml` every time these tests ran (the
/// `canonical_provider_config_wins_over_legacy_alias` fixture happened to
/// deserialize cleanly and overwrite whatever providers the user had set up).
///
/// Our guardrail is the function signature: `migrate_if_needed` now returns
/// `(Self, bool)` and never sees `&self.save()`. This test exercises a
/// migration and asserts we got the side-effect flag back — any future
/// refactor that re-introduces a hidden save would need to either break this
/// contract or ignore the flag, both of which are easier to catch in review.
#[test]
fn migrate_if_needed_returns_save_flag_without_writing() {
    let config_toml = r#"
default_provider = "gemini"

[providers.gemini]
api_key = "should-never-reach-disk"
model = "gemini-3-pro-preview"
"#;
    let config: Config = toml::from_str(config_toml).expect("valid config");
    let (migrated, needs_save) = Config::migrate_if_needed(config);

    assert!(needs_save, "legacy provider must request a save");
    assert_eq!(migrated.default_provider, "google");
    assert!(!migrated.providers.contains_key("gemini"));
}

#[test]
fn migrate_if_needed_skips_save_flag_when_no_migration_needed() {
    let config_toml = r#"
default_provider = "anthropic"

[providers.anthropic]
api_key = "sk-ant-example"
model = "claude-opus-4-6"
"#;
    let config: Config = toml::from_str(config_toml).expect("valid config");
    let (_migrated, needs_save) = Config::migrate_if_needed(config);
    assert!(!needs_save, "no-op migration must not request a save");
}

#[test]
fn project_subagent_model_overrides_personal_without_changing_status_model() {
    let mut personal = Config::default();
    let project_toml = r#"
[providers.openai]
subagent_model = "gpt-5.6-sol"
"#;
    let project: Config = toml::from_str(project_toml).expect("project config");
    let source: toml::Value = toml::from_str(project_toml).expect("project source");
    personal.merge_loaded_project_config(project, &source);
    let provider = personal.providers.get("openai").expect("provider");
    assert_eq!(
        provider.effective_subagent_model(crate::providers::Provider::OpenAI),
        "gpt-5.6-sol"
    );
    assert_eq!(
        provider.effective_fast_model(crate::providers::Provider::OpenAI),
        "gpt-5.6-luna"
    );
}
