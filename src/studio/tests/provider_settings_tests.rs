use super::super::state::SettingsState;
use crate::{config::Config, providers::Provider};

#[test]
fn provider_picker_includes_routed_providers() {
    let config = Config::default();
    let settings = SettingsState::from_config(&config);
    for provider in [Provider::OpenRouter, Provider::Fireworks] {
        assert!(
            settings
                .available_providers
                .iter()
                .any(|name| name == provider.name())
        );
    }
}
