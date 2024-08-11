mod cli;
mod models;
mod relevance;

pub mod prompt;
pub mod service;

pub use cli::handle_gen_command;
pub use service::IrisCommitService;

