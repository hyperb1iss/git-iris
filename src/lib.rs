pub mod cli;
pub mod commands;
pub mod config;
pub mod context;
pub mod file_analyzers;
pub mod git;
pub mod gitmoji;
pub mod interactive;
pub mod llm;
pub mod logger;
pub mod messages;
pub mod prompt;
pub mod llm_providers;
pub mod relevance;
pub mod token_optimizer;
pub mod ui;

// Re-export important structs and functions for easier testing
pub use config::Config;
pub use config::ProviderConfig;
pub use llm_providers::LLMProvider;
pub use prompt::create_prompt;
