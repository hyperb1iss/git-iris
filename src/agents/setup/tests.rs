use super::IrisAgentService;
use crate::config::Config;

#[test]
fn invocation_overrides_preserve_explicit_empty_and_gitmoji_choices() {
    let config = Config {
        instructions: "Saved instruction".into(),
        temp_instructions: Some("Temporary instruction".into()),
        ..Config::default()
    };
    let service = IrisAgentService::new(config, "fireworks".into(), "test".into(), "test".into());
    let inherited = service.invocation_config(None, None, None);
    assert_eq!(
        inherited.temp_instructions.as_deref(),
        Some("Temporary instruction")
    );
    assert_eq!(inherited.instructions, "Saved instruction");
    let overridden = service.invocation_config(Some("conventional"), Some(false), Some(""));
    assert_eq!(overridden.temp_instructions.as_deref(), Some(""));
    assert_eq!(overridden.temp_preset.as_deref(), Some("conventional"));
    assert_eq!(overridden.gitmoji_override, Some(false));
    assert!(!overridden.use_gitmoji);
    assert_eq!(
        service.config.temp_instructions.as_deref(),
        Some("Temporary instruction")
    );
}
