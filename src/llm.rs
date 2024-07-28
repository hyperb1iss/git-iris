use crate::claude_provider::ClaudeProvider;
use crate::config::Config;
use crate::git::GitInfo;
use crate::llm_provider::LLMProvider;
use crate::openai_provider::OpenAIProvider;
use crate::prompt;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

thread_local! {
    pub static PROVIDERS: RefCell<HashMap<String, Arc<dyn LLMProvider>>> = RefCell::new(HashMap::new());
}

pub async fn get_refined_message(
    git_info: &GitInfo,
    config: &Config,
    provider: &str,
    use_gitmoji: bool,
    verbose: bool,
) -> Result<String> {
    let provider_config = config
        .get_provider_config(provider)
        .ok_or_else(|| anyhow!("Provider '{}' not found in configuration", provider))?;

    let provider: Arc<dyn LLMProvider> = PROVIDERS
        .with(|providers| providers.borrow().get(provider).cloned())
        .unwrap_or_else(|| match provider {
            "openai" => Arc::new(OpenAIProvider {
                api_key: provider_config.api_key.clone(),
                model: provider_config.model.clone(),
                additional_params: provider_config.additional_params.clone(),
            }),
            "claude" => Arc::new(ClaudeProvider {
                api_key: provider_config.api_key.clone(),
                model: provider_config.model.clone(),
                additional_params: provider_config.additional_params.clone(),
            }),
            _ => Arc::new(UnsupportedProvider(provider.to_string())),
        });

    if provider.is_unsupported() {
        return Err(anyhow!(
            "Unsupported LLM provider: {}",
            provider.provider_name()
        ));
    }

    let system_prompt = prompt::create_system_prompt(use_gitmoji, &config.custom_instructions);
    let user_prompt = prompt::create_user_prompt(git_info, verbose)?;

    if verbose {
        println!("Using LLM provider: {}", provider.provider_name());
        println!("System prompt:\n{}", system_prompt);
        println!("User prompt:\n{}", user_prompt);
    }

    let refined_message = provider
        .generate_message(&system_prompt, &user_prompt)
        .await?;

    if verbose {
        println!("Generated message:\n{}", refined_message);
    }

    Ok(refined_message)
}

struct UnsupportedProvider(String);

#[async_trait]
impl LLMProvider for UnsupportedProvider {
    async fn generate_message(&self, _system_prompt: &str, _user_prompt: &str) -> Result<String> {
        Err(anyhow!("Unsupported LLM provider: {}", self.0))
    }

    fn is_unsupported(&self) -> bool {
        true
    }

    fn provider_name(&self) -> &str {
        &self.0
    }
}

// This function can be used to initialize providers for testing
pub fn init_providers(providers: HashMap<String, Arc<dyn LLMProvider>>) {
    PROVIDERS.with(|p| {
        *p.borrow_mut() = providers;
    });
}

pub fn clear_providers() {
    PROVIDERS.with(|p| {
        p.borrow_mut().clear();
    });
}
