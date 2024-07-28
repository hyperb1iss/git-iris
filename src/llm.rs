use crate::config::Config;
use crate::git::GitInfo;
use crate::llm_provider::{LLMProvider, LLMProviderManager};
use crate::prompt;
use crate::provider_registry::ProviderRegistry;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

thread_local! {
    pub static PROVIDER_MANAGER: RefCell<LLMProviderManager> = RefCell::new(LLMProviderManager::new());
    pub static PROVIDER_REGISTRY: ProviderRegistry = ProviderRegistry::default();
}

pub async fn get_refined_message(
    git_info: &GitInfo,
    config: &Config,
    provider: &str,
    use_gitmoji: bool,
    verbose: bool,
    inpaint_context: &[String],
    existing_message: Option<&str>,
) -> Result<String> {
    let provider_config = config
        .get_provider_config(provider)
        .ok_or_else(|| anyhow!("Provider '{}' not found in configuration", provider))?;

    let provider: Arc<dyn LLMProvider> = PROVIDER_MANAGER
        .with(|manager| manager.borrow().get_provider(provider).cloned())
        .unwrap_or_else(|| {
            PROVIDER_REGISTRY.with(|registry| {
                let provider_arc = registry
                    .create_provider(provider, provider_config.clone())
                    .unwrap_or_else(|e| {
                        panic!("Failed to create provider {}: {}", provider, e);
                    });
                PROVIDER_MANAGER.with(|manager| {
                    manager
                        .borrow_mut()
                        .register_provider(provider.to_string(), provider_arc.clone());
                });
                provider_arc
            })
        });

    if provider.is_unsupported() {
        return Err(anyhow!(
            "Unsupported LLM provider: {}",
            provider.provider_name()
        ));
    }

    let system_prompt = prompt::create_system_prompt(use_gitmoji, &config.custom_instructions);
    let user_prompt =
        prompt::create_user_prompt(git_info, verbose, inpaint_context, existing_message)?;

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
    PROVIDER_MANAGER.with(|manager| {
        let mut manager = manager.borrow_mut();
        for (name, provider) in providers {
            manager.register_provider(name, provider);
        }
    });
}

pub fn clear_providers() {
    PROVIDER_MANAGER.with(|manager| {
        manager.borrow_mut().clear_providers();
    });
}
