//! TUI module for Git-Iris
//!
//! This module contains the TUI (Text User Interface) implementation for Git-Iris.
//! It provides an interactive interface for users to generate and manage commit messages.

mod app;
mod input_handler;
mod spinner;
mod state;
mod ui;

pub use app::run_tui_commit;
pub use app::TuiCommit;
