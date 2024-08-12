mod changelog;
mod cli;
mod readme_reader;

pub mod change_analyzer;
pub mod prompt;

pub use cli::{handle_changelog_command, handle_release_notes_command};

pub use changelog::{ChangelogGenerator, ReleaseNotesGenerator};