pub mod cli;
pub mod git;
pub mod prompt;
pub mod llm;
pub mod config;
pub mod file_analyzers;
pub mod interactive;

// Re-export important structs and functions for easier testing
pub use git::{GitInfo, FileChange};
pub use config::Config;
pub use prompt::create_prompt;