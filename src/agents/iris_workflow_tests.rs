use super::*;
use crate::{config::Config, providers::Provider};

#[test]
fn delegated_analysis_defaults_to_main_model_and_honors_override() {
    let mut config = Config::default();
    let mut agent = IrisAgent::new("openai", "gpt-6-astra").expect("agent");
    agent.set_fast_model("gpt-5.6-luna".to_string());
    agent.set_config(config.clone());
    assert_eq!(agent.effective_subagent_model(), "gpt-6-astra");
    config
        .providers
        .get_mut("openai")
        .expect("provider")
        .subagent_model = Some("gpt-5.6-sol".to_string());
    agent.set_config(config);
    assert_eq!(agent.effective_subagent_model(), "gpt-5.6-sol");
}

#[test]
fn all_providers_build_complete_agents_and_select_their_own_defaults() {
    for provider in Provider::ALL {
        let mut config = Config::default();
        config.default_provider = provider.name().to_string();
        config
            .providers
            .get_mut(provider.name())
            .expect("provider")
            .api_key = "test-key".to_string();
        let mut agent = IrisAgentBuilder::new()
            .with_provider(provider.name())
            .build()
            .expect("agent");
        assert_eq!(agent.model, provider.default_model());
        agent.set_config(config);
        assert!(agent.build_agent().is_ok(), "provider {provider}");
    }
}
