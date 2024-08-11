pub mod change_analyzer;
pub mod changelog;
pub mod changelog_prompts;
pub mod cli;
pub mod commands;
pub mod config;
pub mod context;
pub mod file_analyzers;
pub mod git;
pub mod gitmoji;
pub mod instruction_presets;
pub mod llm;
pub mod llm_providers;
pub mod logger;
pub mod messages;
pub mod prompt;
pub mod readme_reader;
pub mod relevance;
pub mod service;
pub mod token_optimizer;
pub mod tui;
pub mod ui;

// Re-export important structs and functions for easier testing
pub use config::Config;
pub use config::ProviderConfig;
pub use llm_providers::LLMProvider;
pub use prompt::create_prompt;
