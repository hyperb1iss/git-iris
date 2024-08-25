use super::{LLMProvider, LLMProviderConfig, ProviderMetadata};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::{
    atomic::{AtomicU64, AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Clone)]
pub struct TestLLMProvider {
    config: LLMProviderConfig,
    fail_count: Arc<AtomicUsize>,
    delay: Arc<AtomicU64>,
    total_calls: Arc<AtomicUsize>,
    response: Option<String>,
    bad_response: Option<String>,
    json_validation_failures: Arc<AtomicUsize>,
}

impl TestLLMProvider {
    /// Creates a new instance of `TestLLMProvider` with the given configuration
    pub fn new(config: LLMProviderConfig) -> Self {
        Self {
            config,
            fail_count: Arc::new(AtomicUsize::new(0)),
            delay: Arc::new(AtomicU64::new(0)),
            total_calls: Arc::new(AtomicUsize::new(0)),
            response: None,
            bad_response: None,
            json_validation_failures: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn set_fail_count(&self, count: usize) {
        self.fail_count.store(count, Ordering::SeqCst);
    }

    pub fn set_delay(&self, delay_ms: u64) {
        self.delay.store(delay_ms, Ordering::SeqCst);
    }

    pub fn get_total_calls(&self) -> usize {
        self.total_calls.load(Ordering::SeqCst)
    }

    pub fn set_response(&mut self, response: String) {
        self.response = Some(response);
    }

    pub fn set_bad_response(&mut self, bad_response: String) {
        self.bad_response = Some(bad_response);
    }

    pub fn set_json_validation_failures(&self, count: usize) {
        self.json_validation_failures.store(count, Ordering::SeqCst);
    }

    pub fn reset(&self) {
        self.fail_count.store(0, Ordering::SeqCst);
        self.delay.store(0, Ordering::SeqCst);
        self.total_calls.store(0, Ordering::SeqCst);
        self.json_validation_failures.store(0, Ordering::SeqCst);
    }
}

#[async_trait]
impl LLMProvider for TestLLMProvider {
    /// Generates a message using the Test provider (returns model name + it's own prompts as the message)
    async fn generate_message(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let total_calls = self.total_calls.fetch_add(1, Ordering::SeqCst);
        println!(
            "TestLLMProvider: generate_message called (total calls: {})",
            total_calls + 1
        );

        let delay = self.delay.load(Ordering::SeqCst);
        if delay > 0 {
            println!("TestLLMProvider: Delaying for {delay} ms");
            sleep(Duration::from_millis(delay)).await;
        }

        let fail_count = self.fail_count.load(Ordering::SeqCst);
        if total_calls < fail_count {
            println!("TestLLMProvider: Simulating failure");
            Err(anyhow!("Simulated failure"))
        } else {
            println!("TestLLMProvider: Generating success response");
            let json_validation_failures = self.json_validation_failures.load(Ordering::SeqCst);
            if total_calls < json_validation_failures {
                if let Some(bad_response) = &self.bad_response {
                    Ok(bad_response.clone())
                } else {
                    Err(anyhow!("Simulated JSON validation failure"))
                }
            } else if let Some(response) = &self.response {
                Ok(response.clone())
            } else {
                Ok(format!(
                    "Test response from model '{}'. System prompt: '{}', User prompt: '{}'",
                    self.config.model,
                    system_prompt.replace('\'', "\\'"),
                    user_prompt.replace('\'', "\\'")
                ))
            }
        }
    }
}

pub(super) fn get_metadata() -> ProviderMetadata {
    ProviderMetadata {
        name: "Test",
        default_model: "test-model",
        default_token_limit: 1000,
        requires_api_key: false,
    }
}
