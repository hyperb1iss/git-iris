use super::*;

#[test]
fn test_resolve_api_key_uses_config_when_provided() {
    // Config key takes precedence
    let (key, source) = resolve_api_key(Some("sk-config-key-1234567890"), Provider::OpenAI);
    assert_eq!(key, Some("sk-config-key-1234567890".to_string()));
    assert_eq!(source, ApiKeySource::Config);
}

#[test]
fn test_resolve_api_key_empty_config_not_used() {
    // Empty config should NOT be treated as a valid key
    // It should fall through to env var or client default
    let empty_config: Option<&str> = Some("");
    let (_key, source) = resolve_api_key(empty_config, Provider::OpenAI);

    // Empty config should NOT return Config source
    // This test verifies the empty string is treated as "not configured"
    assert_ne!(source, ApiKeySource::Config);
}

#[test]
fn test_resolve_api_key_none_config_checks_env() {
    // When config is None, should check env var
    let (key, source) = resolve_api_key(None, Provider::OpenAI);

    // Result depends on whether OPENAI_API_KEY is set in the environment
    // We just verify the function doesn't panic and returns appropriate source
    match source {
        ApiKeySource::Environment => {
            assert!(key.is_some());
        }
        ApiKeySource::ClientDefault => {
            assert!(key.is_none());
        }
        ApiKeySource::Config => {
            unreachable!("Should not return Config source when config is None");
        }
    }
}

#[test]
fn test_api_key_source_enum_equality() {
    assert_eq!(ApiKeySource::Config, ApiKeySource::Config);
    assert_eq!(ApiKeySource::Environment, ApiKeySource::Environment);
    assert_eq!(ApiKeySource::ClientDefault, ApiKeySource::ClientDefault);
    assert_ne!(ApiKeySource::Config, ApiKeySource::Environment);
}

#[test]
fn test_resolve_api_key_all_providers() {
    // Test that resolve_api_key works for all supported providers
    for provider in Provider::ALL {
        let (key, source) = resolve_api_key(Some("test-key-123456789012345"), *provider);
        assert_eq!(key, Some("test-key-123456789012345".to_string()));
        assert_eq!(source, ApiKeySource::Config);
    }
}

#[test]
fn test_resolve_api_key_config_precedence() {
    // Even if env var is set, config should take precedence
    // We can't easily mock env vars in unit tests, but we can verify
    // that a provided config key is always used regardless of env state
    let config_key = "sk-from-config-abcdef1234567890";
    let (key, source) = resolve_api_key(Some(config_key), Provider::OpenAI);

    assert_eq!(key.as_deref(), Some(config_key));
    assert_eq!(source, ApiKeySource::Config);
}

#[test]
fn test_api_key_source_debug_impl() {
    // Verify Debug is implemented for logging purposes
    let source = ApiKeySource::Config;
    let debug_str = format!("{:?}", source);
    assert!(debug_str.contains("Config"));
}

#[test]
fn test_apply_completion_params_parses_json_like_additional_params() {
    let mut additional_params = HashMap::new();
    additional_params.insert("temperature".to_string(), "0.7".to_string());
    additional_params.insert("reasoning".to_string(), r#"{"effort":"low"}"#.to_string());

    let params = additional_params_json(Some(&additional_params));
    assert_eq!(params.get("temperature"), Some(&json!(0.7)));
    assert_eq!(params.get("reasoning"), Some(&json!({"effort": "low"})));
}

#[test]
fn test_completion_params_use_profile_specific_openai_reasoning_defaults() {
    let main_params = completion_params_json::<std::collections::hash_map::RandomState>(
        None,
        Provider::OpenAI,
        "gpt-5.4",
        16_384,
        CompletionProfile::MainAgent,
    );
    assert_eq!(
        main_params.get("reasoning"),
        Some(&json!({"effort": "medium"}))
    );
    assert!(!main_params.contains_key("max_output_tokens"));

    let status_params = completion_params_json::<std::collections::hash_map::RandomState>(
        None,
        Provider::OpenAI,
        "gpt-5.4-mini",
        50,
        CompletionProfile::StatusMessage,
    );
    assert_eq!(
        status_params.get("reasoning"),
        Some(&json!({"effort": "none"}))
    );
    assert!(!status_params.contains_key("max_output_tokens"));
}

#[test]
fn test_completion_params_preserve_explicit_reasoning_overrides() {
    let mut additional_params = HashMap::new();
    additional_params.insert("reasoning".to_string(), r#"{"effort":"high"}"#.to_string());

    let params = completion_params_json(
        Some(&additional_params),
        Provider::OpenAI,
        "gpt-5.4",
        4096,
        CompletionProfile::MainAgent,
    );

    assert_eq!(params.get("reasoning"), Some(&json!({"effort": "high"})));
}

#[test]
fn test_completion_params_skip_openai_reasoning_defaults_for_non_gpt5_models() {
    let params = completion_params_json::<std::collections::hash_map::RandomState>(
        None,
        Provider::OpenAI,
        "gpt-4.1",
        4096,
        CompletionProfile::MainAgent,
    );

    assert!(!params.contains_key("reasoning"));
    assert!(!params.contains_key("max_output_tokens"));
}

#[test]
fn test_provider_from_name_supports_aliases() {
    assert_eq!(provider_from_name("openai").ok(), Some(Provider::OpenAI));
    assert_eq!(provider_from_name("claude").ok(), Some(Provider::Anthropic));
    assert_eq!(provider_from_name("gemini").ok(), Some(Provider::Google));
}

mod wire_tests;

#[test]
fn model_profiles_keep_status_cheap_and_subagents_capable() {
    let params = |provider, model, profile| {
        completion_params_json::<std::collections::hash_map::RandomState>(
            None, provider, model, 4096, profile,
        )
    };
    assert_eq!(
        params(Provider::OpenAI, "gpt-6-astra", CompletionProfile::Subagent)["reasoning"]["effort"],
        "low"
    );
    assert_eq!(
        params(
            Provider::OpenAI,
            "gpt-6-astra",
            CompletionProfile::StatusMessage
        )["reasoning"]["effort"],
        "low"
    );
    assert_eq!(
        params(
            Provider::OpenAI,
            "gpt-5.6-luna",
            CompletionProfile::StatusMessage
        )["reasoning"]["effort"],
        "none"
    );
    assert_eq!(
        params(
            Provider::Anthropic,
            "claude-opus-5",
            CompletionProfile::Subagent
        )["output_config"]["effort"],
        "low"
    );
    assert!(
        params(
            Provider::Anthropic,
            "claude-haiku-4-5-20251001",
            CompletionProfile::StatusMessage
        )
        .is_empty()
    );
    assert!(
        params(
            Provider::Google,
            "gemini-3.5-flash-lite",
            CompletionProfile::StatusMessage
        )
        .is_empty()
    );
    assert_eq!(
        params(
            Provider::Fireworks,
            Provider::Fireworks.default_fast_model(),
            CompletionProfile::StatusMessage
        )["reasoning_effort"],
        "none"
    );
    assert!(
        params(
            Provider::Fireworks,
            "accounts/custom/models/custom",
            CompletionProfile::MainAgent
        )
        .is_empty()
    );
}

#[test]
fn native_parameter_overrides_survive_profile_defaults() {
    let params = HashMap::from([
        ("thinking".to_string(), r#"{"type":"disabled"}"#.to_string()),
        (
            "output_config".to_string(),
            r#"{"effort":"max","format":{"type":"json_schema"}}"#.to_string(),
        ),
    ]);
    let result = completion_params_json(
        Some(&params),
        Provider::Anthropic,
        "claude-opus-5",
        4096,
        CompletionProfile::MainAgent,
    );
    assert_eq!(result["thinking"]["type"], "disabled");
    assert_eq!(result["output_config"]["effort"], "max");
    assert_eq!(result["output_config"]["format"]["type"], "json_schema");
}

#[test]
fn provider_registry_drives_configuration_defaults() {
    let config = crate::config::Config::default();
    for provider in [Provider::OpenRouter, Provider::Fireworks] {
        let provider_config = config
            .get_provider_config(provider.name())
            .expect("provider configuration");
        assert_eq!(
            provider_config.effective_subagent_model(provider),
            provider.default_model()
        );
        assert_eq!(
            provider_config.effective_fast_model(provider),
            provider.default_fast_model()
        );
    }
}

#[test]
fn cli_accepts_subagent_models_for_both_config_scopes() {
    use clap::Parser;
    for command in ["config", "project-config"] {
        let cli = crate::cli::Cli::try_parse_from([
            "git-iris",
            command,
            "--provider",
            "openrouter",
            "--subagent-model",
            "openai/gpt-5.6-sol",
        ])
        .expect("configuration args");
        let (crate::cli::Commands::Config { subagent_model, .. }
        | crate::cli::Commands::ProjectConfig { subagent_model, .. }) =
            cli.command.expect("command")
        else {
            panic!("expected config");
        };
        assert_eq!(subagent_model.as_deref(), Some("openai/gpt-5.6-sol"));
    }
}
