mod cli;
mod relevance;

pub mod prompt;
pub mod service;

pub use cli::handle_gen_command;
use git2::FileMode;
pub use service::IrisCommitService;

use crate::git::CommitResult;

pub fn format_commit_result(result: &CommitResult, message: &str) -> String {
    let mut output = format!(
        "[{} {}] {}\n",
        result.branch,
        result.commit_hash,
        message.lines().next().unwrap_or("")
    );
    
    output.push_str(&format!(
        " {} file{} changed, {} insertion{}(+), {} deletion{}(-)\n",
        result.files_changed,
        if result.files_changed != 1 { "s" } else { "" },
        result.insertions,
        if result.insertions != 1 { "s" } else { "" },
        result.deletions,
        if result.deletions != 1 { "s" } else { "" }
    ));
    
    for (file, mode) in &result.new_files {
        output.push_str(&format!(" create mode {} {}\n", format_file_mode(*mode), file));
    }
    
    output
}

fn format_file_mode(mode: FileMode) -> String {
    match mode {
        FileMode::Blob => "100644",
        FileMode::BlobExecutable => "100755",
        FileMode::Link => "120000",
        FileMode::Commit => "160000",
        FileMode::Tree => "040000",
        _ => "000000",
    }.to_string()
}

