//! File reading tool
//!
//! Simple file reading capability with support for partial reads (head/tail).
//! This is more efficient than using `code_search` when you need the actual content.

use anyhow::Result;
use rig::tool::portable::PortableTool;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use crate::define_tool_error;

use super::common::{get_current_repo, parameters_schema};

define_tool_error!(FileReadError);

/// File reading tool for accessing file contents directly
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileRead;

impl FileRead {
    /// Maximum lines to return by default (to avoid overwhelming context)
    const DEFAULT_MAX_LINES: usize = 500;

    /// Line number column width (supports files up to 999,999 lines)
    const LINE_NUM_WIDTH: usize = 6;

    /// Check if file extension indicates binary
    fn is_binary_extension(path: &str) -> bool {
        const BINARY_EXTENSIONS: &[&str] = &[
            ".png", ".jpg", ".jpeg", ".gif", ".bmp", ".ico", ".webp", ".pdf", ".zip", ".tar",
            ".gz", ".rar", ".7z", ".exe", ".dll", ".so", ".dylib", ".bin", ".wasm", ".ttf", ".otf",
            ".woff", ".woff2", ".mp3", ".mp4", ".wav", ".sqlite", ".db", ".pyc", ".class", ".o",
            ".a",
        ];
        let path_lower = path.to_lowercase();
        BINARY_EXTENSIONS
            .iter()
            .any(|ext| path_lower.ends_with(ext))
    }

    /// List directory contents when user accidentally reads a directory
    #[allow(clippy::cast_precision_loss, clippy::as_conversions)] // Fine for human-readable file sizes
    fn list_directory(dir_path: &Path, display_path: &str) -> Result<String, FileReadError> {
        let mut output = String::new();
        output.push_str(&format!("=== {} is a directory ===\n\n", display_path));
        output.push_str("Contents:\n\n");

        let mut entries: Vec<_> = fs::read_dir(dir_path)
            .map_err(|e| FileReadError(format!("Cannot read directory: {e}")))?
            .filter_map(std::result::Result::ok)
            .collect();

        // Sort: directories first, then files, alphabetically within each group
        entries.sort_by(|a, b| {
            let a_is_dir = a.path().is_dir();
            let b_is_dir = b.path().is_dir();
            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.file_name().cmp(&b.file_name()),
            }
        });

        for entry in entries {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            let path = entry.path();

            if path.is_dir() {
                output.push_str(&format!("  📁 {}/\n", name_str));
            } else {
                // Get file size if available
                let size_str = if let Ok(meta) = path.metadata() {
                    let size = meta.len();
                    if size < 1024 {
                        format!("{} B", size)
                    } else if size < 1024 * 1024 {
                        format!("{:.1} KB", size as f64 / 1024.0)
                    } else {
                        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
                    }
                } else {
                    String::new()
                };
                output.push_str(&format!("  📄 {}  ({})\n", name_str, size_str));
            }
        }

        output.push_str("\nUse file_read with a specific file path to read contents.\n");
        Ok(output)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FileReadArgs {
    /// Path to the file to read (relative to repo root)
    pub path: String,
    /// Starting line number (1-indexed, default: 1)
    #[serde(default)]
    pub start_line: Option<usize>,
    /// Number of lines to read (default: 500, max: 1000)
    #[serde(default)]
    pub num_lines: Option<usize>,
}

impl PortableTool for FileRead {
    const NAME: &'static str = "file_read";
    type Error = FileReadError;
    type Args = FileReadArgs;
    type Output = String;

    fn description(&self) -> String {
        "Read file contents directly. Use start_line and num_lines for partial reads on large files. Returns line-numbered content.".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        parameters_schema::<FileReadArgs>()
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "Defer synchronous tool work until polling inside the repository context"
    )]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let repo = get_current_repo().map_err(FileReadError::from)?;
        let repo_path = repo.repo_path();

        // Reject absolute paths - all paths must be relative to repo root
        if Path::new(&args.path).is_absolute() {
            return Err(FileReadError(
                "Absolute paths not allowed. Use paths relative to repository root.".into(),
            ));
        }

        // Join path to repo root
        let file_path = repo_path.join(&args.path);

        // Check file exists before canonicalization
        if !file_path.exists() {
            return Err(FileReadError(format!("File not found: {}", args.path)));
        }

        // Canonicalize both paths to resolve symlinks and .. components
        let canonical_file = file_path
            .canonicalize()
            .map_err(|e| FileReadError(format!("Cannot resolve path: {e}")))?;
        let canonical_repo = repo_path
            .canonicalize()
            .map_err(|e| FileReadError(format!("Cannot resolve repo path: {e}")))?;

        // Security: verify resolved path is within repository bounds
        if !canonical_file.starts_with(&canonical_repo) {
            return Err(FileReadError("Path escapes repository boundaries".into()));
        }

        // If it's a directory, return a helpful listing instead of an error
        if canonical_file.is_dir() {
            return Self::list_directory(&canonical_file, &args.path);
        }

        if !canonical_file.is_file() {
            return Err(FileReadError(format!("Not a file: {}", args.path)));
        }

        // Check for binary extension
        if Self::is_binary_extension(&args.path) {
            return Ok(format!(
                "[Binary file: {} - content not displayed]",
                args.path
            ));
        }

        Self::read_excerpt(&canonical_file, &args)
    }
}

impl FileRead {
    pub(super) fn read_excerpt(path: &Path, args: &FileReadArgs) -> Result<String, FileReadError> {
        const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;
        let file = fs::File::open(path).map_err(|e| FileReadError(e.to_string()))?;
        let is_partial_read = args.start_line.is_some() || args.num_lines.is_some();
        let file_len = file
            .metadata()
            .map_err(|e| FileReadError(e.to_string()))?
            .len();

        if !is_partial_read && file_len > MAX_FILE_SIZE {
            #[allow(clippy::cast_precision_loss, clippy::as_conversions)]
            let size_mb = file_len as f64 / (1024.0 * 1024.0);
            return Err(FileReadError(format!(
                "File too large ({size_mb:.1} MB). Max is 10 MB. Use start_line/num_lines for partial reads.",
            )));
        }

        let mut reader = BufReader::new(file);
        if reader
            .fill_buf()
            .map_err(|e| FileReadError(e.to_string()))?
            .contains(&0)
        {
            return Ok(format!(
                "[Binary file detected: {} - content not displayed]",
                args.path
            ));
        }

        let requested_start = args.start_line.unwrap_or(1).saturating_sub(1);
        let max_lines = args.num_lines.unwrap_or(Self::DEFAULT_MAX_LINES).min(1000);
        let requested_end = requested_start.saturating_add(max_lines);
        let mut total_lines = 0;
        let mut content = Vec::new();
        loop {
            // Skipped lines never allocate, even when a generated file has a giant line.
            let bytes = if (requested_start..requested_end).contains(&total_lines) {
                let retained =
                    u64::try_from(content.len()).map_err(|e| FileReadError(e.to_string()))?;
                let remaining = MAX_FILE_SIZE.saturating_sub(retained);
                let bytes = reader
                    .by_ref()
                    .take(remaining + 1)
                    .read_until(b'\n', &mut content)
                    .map_err(|e| FileReadError(e.to_string()))?;
                if u64::try_from(content.len()).map_err(|e| FileReadError(e.to_string()))?
                    > MAX_FILE_SIZE
                {
                    return Err(FileReadError(
                        "Selected excerpt exceeds 10 MB. Request fewer lines.".into(),
                    ));
                }
                bytes
            } else {
                reader
                    .skip_until(b'\n')
                    .map_err(|e| FileReadError(e.to_string()))?
            };
            if bytes == 0 {
                break;
            }
            total_lines += 1;
        }
        if content.contains(&0) {
            return Ok(format!(
                "[Binary file detected: {} - content not displayed]",
                args.path
            ));
        }
        let content_str = String::from_utf8(content).map_err(|e| FileReadError(e.to_string()))?;
        let start = requested_start.min(total_lines);
        let end = requested_end.min(total_lines);

        // Build output with line numbers
        let mut output = String::new();
        output.push_str(&format!(
            "=== {} ({} total lines) ===\n",
            args.path, total_lines
        ));

        if start == end {
            output.push_str("No lines in requested range.\n");
        } else if start > 0 || end < total_lines {
            output.push_str(&format!(
                "Showing lines {}-{} of {}\n",
                start + 1,
                end,
                total_lines
            ));
        }
        output.push('\n');

        for (i, line) in content_str.lines().enumerate() {
            output.push_str(&format!(
                "{:>width$}│ {}\n",
                start + i + 1,
                line,
                width = Self::LINE_NUM_WIDTH
            ));
        }

        if end < total_lines {
            output.push_str(&format!(
                "\n... {} more lines (use start_line={} to continue)\n",
                total_lines - end,
                end + 1
            ));
        }

        Ok(output)
    }
}
