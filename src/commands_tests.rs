use super::apply_provider_settings;
use crate::{common::CommonParams, config::Config};

#[test]
fn project_provider_aliases_store_overrides_under_the_canonical_name() {
    for alias in ["claude", "Anthropic"] {
        let mut config = Config::default();
        let common = CommonParams {
            provider: Some(alias.to_string()),
            ..CommonParams::default()
        };
        let mut changed = false;
        let name = apply_provider_settings(
            &mut config,
            &common,
            Some("primary".into()),
            Some("status".into()),
            Some("worker".into()),
            None,
            None,
            &mut changed,
        )
        .expect("valid alias");
        assert_eq!(name, "anthropic");
        assert!(changed);
        assert!(!config.providers.contains_key(alias));
        let provider = &config.providers["anthropic"];
        assert_eq!(provider.model, "primary");
        assert_eq!(provider.fast_model.as_deref(), Some("status"));
        assert_eq!(provider.subagent_model.as_deref(), Some("worker"));
    }
}
