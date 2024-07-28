pub mod claude_provider;
pub mod cli;
pub mod commands;
pub mod config;
pub mod file_analyzers;
pub mod git;
pub mod interactive;
pub mod llm;
pub mod llm_provider;
pub mod openai_provider;
pub mod prompt;
pub mod provider_registry;

// Re-export important structs and functions for easier testing
pub use config::Config;
pub use config::ProviderConfig;
pub use git::{FileChange, GitInfo};
pub use prompt::create_prompt;
