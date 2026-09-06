//! Static analysis tool for agent review context.

use anyhow::Result;
use rig::tool::portable::PortableTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

use super::common::{current_repo_execution_trusted, current_repo_root, parameters_schema};

crate::define_tool_error!(StaticAnalysisError);

const DEFAULT_TIMEOUT_SECS: u64 = 300;
const DEFAULT_MAX_OUTPUT_CHARS: usize = 12_000;
const MIN_OUTPUT_CHARS: usize = 512;
const MAX_TIMEOUT_SECS: u64 = 600;
const MAX_OUTPUT_CHARS: usize = 40_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticAnalysis;

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StaticAnalyzer {
    #[default]
    Auto,
    Rust,
    Python,
    Javascript,
    Go,
}

impl fmt::Display for StaticAnalyzer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Rust => write!(f, "rust"),
            Self::Python => write!(f, "python"),
            Self::Javascript => write!(f, "javascript"),
            Self::Go => write!(f, "go"),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct StaticAnalysisArgs {
    #[serde(default)]
    pub analyzer: StaticAnalyzer,
    #[serde(default = "default_timeout_secs")]
    #[schemars(
        description = "Seconds to wait per analysis command. Values are clamped to 1..600."
    )]
    pub timeout_secs: u64,
    #[serde(default = "default_max_output_chars")]
    #[schemars(
        description = "Maximum characters to return per command. Values are clamped to 512..40000."
    )]
    pub max_output_chars: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AnalysisCommand {
    pub(super) name: &'static str,
    pub(super) executable: &'static str,
    pub(super) args: Vec<&'static str>,
    pub(super) reason: &'static str,
}

impl Default for StaticAnalysis {
    fn default() -> Self {
        Self
    }
}

fn default_timeout_secs() -> u64 {
    DEFAULT_TIMEOUT_SECS
}

fn default_max_output_chars() -> usize {
    DEFAULT_MAX_OUTPUT_CHARS
}

impl PortableTool for StaticAnalysis {
    const NAME: &'static str = "static_analysis";
    type Error = StaticAnalysisError;
    type Args = StaticAnalysisArgs;
    type Output = String;

    fn description(&self) -> String {
        "Run installed static analysis tools directly without performing package install steps. Supports Rust/clippy, Python/ruff, JavaScript or TypeScript/biome or oxlint, and Go/golangci-lint or go vet. Use this during review to prioritize analyzer findings and avoid reporting issues a linter already catches. These tools can execute project build scripts, plugins, or analyzer configuration, so only run them in trusted workspaces. Timeouts clamp to 1..=600 seconds; output truncates to 512..=40000 characters.".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        parameters_schema::<StaticAnalysisArgs>()
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        if !current_repo_execution_trusted() {
            return Err(StaticAnalysisError(
                "Static analysis cannot execute project code in an untrusted remote clone. Review it with the read-only tools, or inspect and clone the repository locally before running analysis."
                    .to_string(),
            ));
        }
        let repo_root = current_repo_root()?;
        let commands = select_analysis_commands(&repo_root, args.analyzer, command_available);
        if commands.is_empty() {
            let availability =
                unavailable_analysis_summary(&repo_root, args.analyzer, command_available);
            return Ok(format!(
                "No installed static analysis command found for `{}`. Supported direct commands: cargo, ruff, biome, oxlint, golangci-lint, go.\n{}",
                args.analyzer,
                availability.join("\n")
            ));
        }

        let timeout_secs = args.timeout_secs.clamp(1, MAX_TIMEOUT_SECS);
        let max_output_chars = args
            .max_output_chars
            .clamp(MIN_OUTPUT_CHARS, MAX_OUTPUT_CHARS);
        let mut output = format!(
            "Static analysis: {} command(s), timeout {}s each\n",
            commands.len(),
            timeout_secs
        );

        for command in commands {
            output.push_str(&format!(
                "\n## {}\nReason: {}\nCommand: {} {}\n",
                command.name,
                command.reason,
                command.executable,
                command.args.join(" ")
            ));
            output.push_str(
                &run_analysis_command(&repo_root, &command, timeout_secs, max_output_chars).await,
            );
            output.push('\n');
        }

        Ok(output)
    }
}

pub(super) fn select_analysis_commands(
    repo_root: &Path,
    analyzer: StaticAnalyzer,
    command_available: impl Fn(&str) -> bool,
) -> Vec<AnalysisCommand> {
    let mut commands = Vec::new();
    let wants = |candidate| analyzer == StaticAnalyzer::Auto || analyzer == candidate;

    if wants(StaticAnalyzer::Rust)
        && (analyzer == StaticAnalyzer::Rust || repo_root.join("Cargo.toml").is_file())
        && command_available("cargo")
    {
        commands.push(AnalysisCommand {
            name: "Rust clippy",
            executable: "cargo",
            args: vec![
                "clippy",
                "--workspace",
                "--no-deps",
                "--message-format",
                "short",
            ],
            reason: if analyzer == StaticAnalyzer::Rust {
                "Rust analyzer requested"
            } else {
                "Cargo.toml detected"
            },
        });
    }

    if wants(StaticAnalyzer::Python)
        && (analyzer == StaticAnalyzer::Python
            || has_any(
                repo_root,
                &["pyproject.toml", "ruff.toml", ".ruff.toml", "setup.cfg"],
            ))
        && command_available("ruff")
    {
        commands.push(AnalysisCommand {
            name: "Python ruff",
            executable: "ruff",
            args: vec!["check", "."],
            reason: if analyzer == StaticAnalyzer::Python {
                "Python analyzer requested"
            } else {
                "Python project config detected"
            },
        });
    }

    if wants(StaticAnalyzer::Javascript)
        && (analyzer == StaticAnalyzer::Javascript || repo_root.join("package.json").is_file())
    {
        if command_available("biome") {
            commands.push(AnalysisCommand {
                name: "JavaScript/TypeScript biome",
                executable: "biome",
                args: vec!["check", "."],
                reason: if analyzer == StaticAnalyzer::Javascript {
                    "JavaScript analyzer requested and biome is installed"
                } else {
                    "package.json detected and biome is installed"
                },
            });
        } else if command_available("oxlint") {
            commands.push(AnalysisCommand {
                name: "JavaScript/TypeScript oxlint",
                executable: "oxlint",
                args: vec!["."],
                reason: if analyzer == StaticAnalyzer::Javascript {
                    "JavaScript analyzer requested and oxlint is installed"
                } else {
                    "package.json detected and oxlint is installed"
                },
            });
        }
    }

    if wants(StaticAnalyzer::Go)
        && (analyzer == StaticAnalyzer::Go || repo_root.join("go.mod").is_file())
    {
        if command_available("golangci-lint") {
            commands.push(AnalysisCommand {
                name: "Go golangci-lint",
                executable: "golangci-lint",
                args: vec!["run"],
                reason: if analyzer == StaticAnalyzer::Go {
                    "Go analyzer requested and golangci-lint is installed"
                } else {
                    "go.mod detected and golangci-lint is installed"
                },
            });
        } else if command_available("go") {
            commands.push(AnalysisCommand {
                name: "Go vet",
                executable: "go",
                args: vec!["vet", "./..."],
                reason: if analyzer == StaticAnalyzer::Go {
                    "Go analyzer requested and go is installed"
                } else {
                    "go.mod detected and go is installed"
                },
            });
        }
    }

    commands
}

pub(super) fn unavailable_analysis_summary(
    repo_root: &Path,
    analyzer: StaticAnalyzer,
    command_available: impl Fn(&str) -> bool,
) -> Vec<String> {
    let mut notes = Vec::new();
    let wants = |candidate| analyzer == StaticAnalyzer::Auto || analyzer == candidate;
    let python_config = has_any(
        repo_root,
        &["pyproject.toml", "ruff.toml", ".ruff.toml", "setup.cfg"],
    );
    let mut applicable_auto_marker = false;

    if wants(StaticAnalyzer::Rust) {
        let applicable = analyzer == StaticAnalyzer::Rust || repo_root.join("Cargo.toml").is_file();
        applicable_auto_marker |= analyzer == StaticAnalyzer::Auto && applicable;
        if applicable && !command_available("cargo") {
            notes.push(if analyzer == StaticAnalyzer::Rust {
                "Rust analyzer requested but cargo is not on PATH.".to_string()
            } else {
                "Cargo.toml detected but cargo is not on PATH.".to_string()
            });
        }
    }

    if wants(StaticAnalyzer::Python) {
        let applicable = analyzer == StaticAnalyzer::Python || python_config;
        applicable_auto_marker |= analyzer == StaticAnalyzer::Auto && applicable;
        if applicable && !command_available("ruff") {
            notes.push(if analyzer == StaticAnalyzer::Python {
                "Python analyzer requested but ruff is not on PATH.".to_string()
            } else {
                "Python config detected but ruff is not on PATH.".to_string()
            });
        }
    }

    if wants(StaticAnalyzer::Javascript) {
        let applicable =
            analyzer == StaticAnalyzer::Javascript || repo_root.join("package.json").is_file();
        applicable_auto_marker |= analyzer == StaticAnalyzer::Auto && applicable;
        if applicable && !command_available("biome") && !command_available("oxlint") {
            notes.push(if analyzer == StaticAnalyzer::Javascript {
                "JavaScript analyzer requested but biome and oxlint are not on PATH.".to_string()
            } else {
                "package.json detected but biome and oxlint are not on PATH.".to_string()
            });
        }
    }

    if wants(StaticAnalyzer::Go) {
        let applicable = analyzer == StaticAnalyzer::Go || repo_root.join("go.mod").is_file();
        applicable_auto_marker |= analyzer == StaticAnalyzer::Auto && applicable;
        if applicable && !command_available("golangci-lint") && !command_available("go") {
            notes.push(if analyzer == StaticAnalyzer::Go {
                "Go analyzer requested but golangci-lint and go are not on PATH.".to_string()
            } else {
                "go.mod detected but golangci-lint and go are not on PATH.".to_string()
            });
        }
    }

    if notes.is_empty() && analyzer == StaticAnalyzer::Auto && !applicable_auto_marker {
        notes.push("No matching project markers detected for auto mode.".to_string());
    }

    notes
}

fn has_any(repo_root: &Path, names: &[&str]) -> bool {
    names.iter().any(|name| repo_root.join(name).is_file())
}

fn command_available(command: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|path| executable_exists(&path.join(command)))
    })
}

pub(super) fn executable_exists(path: &Path) -> bool {
    #[cfg(windows)]
    {
        if path.is_file() {
            return true;
        }

        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return false;
        };
        let pathext =
            std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
        pathext
            .split(';')
            .filter(|extension| !extension.is_empty())
            .any(|extension| path.with_file_name(format!("{name}{extension}")).is_file())
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        path.metadata()
            .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
    }

    #[cfg(not(any(unix, windows)))]
    {
        path.is_file()
    }
}

async fn run_analysis_command(
    repo_root: &Path,
    command: &AnalysisCommand,
    timeout_secs: u64,
    max_output_chars: usize,
) -> String {
    let mut process = Command::new(command.executable);
    process.args(&command.args);
    process.current_dir(repo_root);
    process.stdin(Stdio::null());
    process.stdout(Stdio::piped());
    process.stderr(Stdio::piped());
    process.kill_on_drop(true);

    match timeout(Duration::from_secs(timeout_secs), process.output()).await {
        Ok(Ok(output)) => format_command_output(output.status.success(), &output, max_output_chars),
        Ok(Err(error)) => format!("Failed to run {}: {error}\n", command.executable),
        Err(_) => format!("Timed out after {timeout_secs}s\n"),
    }
}

fn format_command_output(success: bool, output: &std::process::Output, max_chars: usize) -> String {
    let status = if success { "passed" } else { "failed" };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("Status: {status}\n\nstderr:\n{stderr}\n\nstdout:\n{stdout}");
    truncate_chars(&combined, max_chars)
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let mut truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_none() {
        return truncated;
    }

    truncated.push_str("\n[static_analysis output truncated]");
    truncated
}
