# Adding Tools

Tools give Iris the ability to inspect your codebase, gather context, and perform operations. This guide shows you how to implement a new tool using the Rig framework.

## What is a Tool?

A tool is a Rust struct that implements the `rig::tool::Tool` trait. When Iris needs information, she can invoke tools by name with specific arguments. The tool executes and returns structured data.

### Tool Lifecycle

```
1. Iris decides she needs information
2. Iris calls tool by name: git_diff(from="<default-branch>", to="HEAD")
3. Tool executes and returns structured output
4. Iris incorporates the result into her reasoning
5. Iris may call more tools or produce final output
```

## Tool Trait Requirements

Repository tools implement Rig 0.42's `PortableTool`; Rig adapts it to the runtime tool interface:

```rust
use rig::tool::portable::PortableTool;

impl PortableTool for MyTool {
    const NAME: &'static str = "my_tool";
    type Error = MyToolError;
    type Args = MyToolArgs;
    type Output = String;

    fn description(&self) -> String {
        "Describe what the tool does".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        parameters_schema::<MyToolArgs>()
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Execute tool logic
    }
}
```

`Output` does not have to be `String`. Any type that implements `Serialize` and `Deserialize` works — Rig will serialize the value when it relays the tool result to the LLM. The canonical modern reference for a struct output is `src/agents/tools/repo_map.rs`, where `RepoMapTool` uses `type Output = RepoMap;` and returns a structured `RepoMap` (`pub struct RepoMap { pub files_analyzed: usize, pub files_shown: usize, pub changed_files: Vec<PathBuf>, pub mentioned_files: Vec<PathBuf>, pub content: String }`). Use `String` when the output is naturally a single rendered blob (most git tools); use a struct when the consumer benefits from typed fields.

## Step-by-Step: Creating a Tool

### Example: Dependency Analyzer

Let's build a tool that analyzes project dependencies.

### Step 1: Create the Tool File

Create `src/agents/tools/dependency_analyzer.rs`:

```rust
//! Dependency analyzer tool for Iris
//!
//! Analyzes project dependencies from package manifests.

use anyhow::Result;
use rig::tool::portable::PortableTool;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::common::parameters_schema;

// Define error type using the standard macro
crate::define_tool_error!(DependencyAnalyzerError);

/// Dependency analyzer tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyAnalyzer;

/// Arguments for dependency analysis
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct DependencyAnalyzerArgs {
    /// Type of manifest to analyze (cargo, npm, pip, etc.)
    #[serde(default)]
    pub manifest_type: Option<String>,
    /// Whether to include dev dependencies
    #[serde(default)]
    pub include_dev: bool,
}

impl PortableTool for DependencyAnalyzer {
    const NAME: &'static str = "dependency_analyzer";
    type Error = DependencyAnalyzerError;
    type Args = DependencyAnalyzerArgs;
    type Output = String;

    fn description(&self) -> String {
        "Analyze project dependencies from package manifests (Cargo.toml, package.json, requirements.txt)".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        parameters_schema::<DependencyAnalyzerArgs>()
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Get current working directory
        let repo_path = super::common::current_repo_root()
            .map_err(|e| DependencyAnalyzerError(format!("Failed to get CWD: {}", e)))?;

        // Detect manifest type if not specified
        let manifest_type = match args.manifest_type.as_deref() {
            Some(t) => t.to_string(),
            None => detect_manifest_type(&repo_path)?,
        };

        // Read and parse manifest
        let dependencies = match manifest_type.as_str() {
            "cargo" => parse_cargo_toml(&repo_path, args.include_dev)?,
            "npm" => parse_package_json(&repo_path, args.include_dev)?,
            "pip" => parse_requirements_txt(&repo_path)?,
            _ => return Err(DependencyAnalyzerError(format!(
                "Unsupported manifest type: {}",
                manifest_type
            ))),
        };

        Ok(dependencies)
    }
}

/// Detect manifest type from files present
fn detect_manifest_type(repo_path: &PathBuf) -> Result<String, DependencyAnalyzerError> {
    if repo_path.join("Cargo.toml").exists() {
        Ok("cargo".to_string())
    } else if repo_path.join("package.json").exists() {
        Ok("npm".to_string())
    } else if repo_path.join("requirements.txt").exists() {
        Ok("pip".to_string())
    } else {
        Err(DependencyAnalyzerError(
            "No recognized dependency manifest found".to_string(),
        ))
    }
}

/// Parse Cargo.toml
fn parse_cargo_toml(
    repo_path: &PathBuf,
    include_dev: bool,
) -> Result<String, DependencyAnalyzerError> {
    use std::fs;

    let cargo_path = repo_path.join("Cargo.toml");
    let content = fs::read_to_string(&cargo_path)
        .map_err(|e| DependencyAnalyzerError(format!("Failed to read Cargo.toml: {}", e)))?;

    let cargo_toml: toml::Value = toml::from_str(&content)
        .map_err(|e| DependencyAnalyzerError(format!("Failed to parse Cargo.toml: {}", e)))?;

    let mut output = String::from("## Rust Dependencies (Cargo.toml)\n\n");

    // Regular dependencies
    if let Some(deps) = cargo_toml.get("dependencies").and_then(|v| v.as_table()) {
        output.push_str("### Dependencies\n");
        for (name, value) in deps {
            let version = extract_version(value);
            output.push_str(&format!("- {} = {}\n", name, version));
        }
        output.push('\n');
    }

    // Dev dependencies
    if include_dev {
        if let Some(dev_deps) = cargo_toml.get("dev-dependencies").and_then(|v| v.as_table()) {
            output.push_str("### Dev Dependencies\n");
            for (name, value) in dev_deps {
                let version = extract_version(value);
                output.push_str(&format!("- {} = {}\n", name, version));
            }
        }
    }

    Ok(output)
}

/// Parse package.json
fn parse_package_json(
    repo_path: &PathBuf,
    include_dev: bool,
) -> Result<String, DependencyAnalyzerError> {
    use std::fs;

    let package_path = repo_path.join("package.json");
    let content = fs::read_to_string(&package_path)
        .map_err(|e| DependencyAnalyzerError(format!("Failed to read package.json: {}", e)))?;

    let package: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| DependencyAnalyzerError(format!("Failed to parse package.json: {}", e)))?;

    let mut output = String::from("## JavaScript/TypeScript Dependencies (package.json)\n\n");

    // Regular dependencies
    if let Some(deps) = package.get("dependencies").and_then(|v| v.as_object()) {
        output.push_str("### Dependencies\n");
        for (name, value) in deps {
            let version = value.as_str().unwrap_or("*");
            output.push_str(&format!("- {} @ {}\n", name, version));
        }
        output.push('\n');
    }

    // Dev dependencies
    if include_dev {
        if let Some(dev_deps) = package.get("devDependencies").and_then(|v| v.as_object()) {
            output.push_str("### Dev Dependencies\n");
            for (name, value) in dev_deps {
                let version = value.as_str().unwrap_or("*");
                output.push_str(&format!("- {} @ {}\n", name, version));
            }
        }
    }

    Ok(output)
}

/// Parse requirements.txt
fn parse_requirements_txt(repo_path: &PathBuf) -> Result<String, DependencyAnalyzerError> {
    use std::fs;

    let req_path = repo_path.join("requirements.txt");
    let content = fs::read_to_string(&req_path).map_err(|e| {
        DependencyAnalyzerError(format!("Failed to read requirements.txt: {}", e))
    })?;

    let mut output = String::from("## Python Dependencies (requirements.txt)\n\n");

    for line in content.lines() {
        let trimmed = line.trim();
        // Skip comments and empty lines
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            output.push_str(&format!("- {}\n", trimmed));
        }
    }

    Ok(output)
}

/// Extract version from TOML value (handles both string and table formats)
fn extract_version(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => s.clone(),
        toml::Value::Table(t) => t
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("*")
            .to_string(),
        _ => "*".to_string(),
    }
}
```

### Step 2: Add to Module Exports

Edit `src/agents/tools/mod.rs`:

```rust
pub mod dependency_analyzer;
pub use dependency_analyzer::DependencyAnalyzer;
```

### Step 3: Register in Tool Registry

Edit `src/agents/tools/registry.rs`. Add your tool to both the `attach_core_tools!` macro body and the `CORE_TOOLS` reference slice. The current registry attaches the 11 shipped core tools:

```rust
#[macro_export]
macro_rules! attach_core_tools {
    ($builder:expr) => {{
        use $crate::agents::debug_tool::DebugTool;
        use $crate::agents::tools::{
            CodeSearch, DependencyAnalyzer, FileRead, GitBlame, GitChangedFiles, GitDiff, GitLog,
            GitShow, GitStatus, ProjectDocs, RepoMapTool, StaticAnalysis,
        };

        $builder
            .tool(DebugTool::new(GitStatus))
            .tool(DebugTool::new(GitDiff))
            .tool(DebugTool::new(GitLog))
            .tool(DebugTool::new(GitShow))
            .tool(DebugTool::new(GitChangedFiles))
            .tool(DebugTool::new(GitBlame))
            .tool(DebugTool::new(FileRead))
            .tool(DebugTool::new(CodeSearch))
            .tool(DebugTool::new(RepoMapTool))
            .tool(DebugTool::new(StaticAnalysis))
            .tool(DebugTool::new(ProjectDocs))
            .tool(DebugTool::new(DependencyAnalyzer))  // Add here
    }};
}

pub const CORE_TOOLS: &[&str] = &[
    "git_status",
    "git_diff",
    "git_log",
    "git_show",
    "git_changed_files",
    "git_blame",
    "file_read",
    "code_search",
    "repo_map",
    "static_analysis",
    "project_docs",
    "dependency_analyzer",  // Add here
];
```

The `registry.rs` test asserts `CORE_TOOLS.len()` matches the count of attached tools — bump it when you add an entry.

### Step 4: Test Your Tool

```bash
# Build
just build

# Test with debug mode to see tool calls
just gen-debug

# Run a specific test by name
just test-one dependency_analyzer
```

## Tool Design Patterns

### Pattern 1: Simple Query Tool

Returns information based on arguments:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleQueryTool;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SimpleQueryArgs {
    pub query: String,
}

impl PortableTool for SimpleQueryTool {
    const NAME: &'static str = "simple_query";
    type Error = SimpleQueryError;
    type Args = SimpleQueryArgs;
    type Output = String;

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Process query and return results
        Ok(format!("Results for: {}", args.query))
    }
}
```

### Pattern 2: Stateful Tool

Maintains internal state across calls:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatefulTool {
    #[serde(skip)]
    state: Arc<Mutex<ToolState>>,
}

#[derive(Debug, Default)]
struct ToolState {
    cache: HashMap<String, String>,
}

impl StatefulTool {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ToolState::default())),
        }
    }
}

impl PortableTool for StatefulTool {
    // ... implementation uses self.state
}
```

**Example**: `Workspace` tool (see `src/agents/tools/workspace.rs`)

### Pattern 3: Repository-Aware Tool

Accesses Git repository data:

```rust
use crate::git::GitRepo;
use super::common::get_current_repo;

impl PortableTool for GitAwareTool {
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let repo = get_current_repo().map_err(GitAwareError::from)?;

        // Use repo methods
        let branch = repo.get_current_branch()?;
        let files = repo.extract_files_info(false)?;

        // Process and return
        Ok(format!("Branch: {}, Files: {}", branch, files.staged_files.len()))
    }
}
```

**Example**: `GitDiff`, `GitLog`, `GitStatus` (see `src/agents/tools/git.rs`)

### Pattern 4: File System Tool

Reads files and analyzes content:

```rust
use std::fs;
use std::path::PathBuf;

impl PortableTool for FileSystemTool {
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = PathBuf::from(&args.file_path);

        // Read file
        let content = fs::read_to_string(&path)
            .map_err(|e| FileSystemError(format!("Failed to read file: {}", e)))?;

        // Analyze
        let line_count = content.lines().count();

        Ok(format!("File has {} lines", line_count))
    }
}
```

**Example**: `FileRead` (see `src/agents/tools/file_read.rs`)

## Best Practices

### 1. Clear Tool Descriptions

The `description()` method supplies the text Iris sees. Make it actionable:

```rust
fn description(&self) -> String {
    "Analyze project dependencies from package manifests. Auto-detects Cargo.toml, package.json, or requirements.txt. Use include_dev=true for dev dependencies.".to_string()
}
```

**Good**: "Analyze project dependencies from package manifests"
**Bad**: "A tool for dependencies"

### 2. Useful Default Arguments

Use `#[serde(default)]` for optional arguments:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct MyToolArgs {
    /// Required query
    pub query: String,

    /// Optional limit (defaults to 10)
    #[serde(default = "default_limit")]
    pub limit: usize,

    /// Optional flag (defaults to false)
    #[serde(default)]
    pub include_extra: bool,
}

fn default_limit() -> usize {
    10
}
```

### 3. Structured Output

Return data in a format Iris can parse and reason about:

```rust
// Good - structured sections
Ok(format!(
    "## Summary\n{}\n\n## Details\n{}\n\n## Recommendations\n{}",
    summary, details, recommendations
))

// Bad - unstructured text
Ok(format!("{} {} {}", summary, details, recommendations))
```

### 4. Error Handling

Use descriptive errors:

```rust
// Good
Err(DependencyAnalyzerError(format!(
    "No package.json found in {}. Make sure you're in a Node.js project.",
    repo_path.display()
)))

// Bad
Err(DependencyAnalyzerError("File not found".to_string()))
```

### 5. Performance Considerations

**Cache expensive operations:**

```rust
#[derive(Debug, Clone)]
pub struct CachedTool {
    #[serde(skip)]
    cache: Arc<Mutex<HashMap<String, String>>>,
}

impl PortableTool for CachedTool {
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let mut cache = self.cache.lock().unwrap();

        if let Some(cached) = cache.get(&args.query) {
            return Ok(cached.clone());
        }

        let result = expensive_operation(&args.query)?;
        cache.insert(args.query.clone(), result.clone());
        Ok(result)
    }
}
```

**Limit output size:**

```rust
// Truncate large outputs
let mut output = generate_output();
if output.len() > 10_000 {
    output.truncate(10_000);
    output.push_str("\n\n... (output truncated)");
}
Ok(output)
```

### 6. Test Your Tool

Per the [project test convention](./contributing.md#testing-requirements), tests live in a `tests/` subdirectory alongside the module they exercise — never inline in the tool's own `.rs` file. The shipped tool tests follow this pattern: `src/agents/tools/tests/{repo_map_tests.rs, git_blame_tests.rs, git_show_tests.rs, static_analysis_tests.rs}`.

To add tests for `dependency_analyzer`:

1. Create `src/agents/tools/tests/dependency_analyzer_tests.rs`:

   ```rust
   use rig::tool::portable::PortableTool;

   use crate::agents::tools::dependency_analyzer::{
       DependencyAnalyzer, DependencyAnalyzerArgs,
   };

   #[tokio::test]
   async fn test_dependency_analyzer_cargo() {
       let tool = DependencyAnalyzer;
       let args = DependencyAnalyzerArgs {
           manifest_type: Some("cargo".to_string()),
           include_dev: false,
       };

       let result = tool.call(args).await;
       assert!(result.is_ok());
   }
   ```

2. Declare the test module in `src/agents/tools/tests/mod.rs`:

   ```rust
   mod dependency_analyzer_tests;
   ```

`src/agents/tools/mod.rs` already ends with `#[cfg(test)] mod tests;`, so the new test file is picked up automatically.

## Real-World Examples

### Git Diff Tool

From `src/agents/tools/git.rs`:

**Key features:**

- Multiple detail levels (`summary`, `standard`)
- Relevance scoring to prioritize files
- Size guidance for agents
- Flexible ref arguments

**Study this for**: Repository operations, scoring algorithms, output formatting

### File Read Tool

From `src/agents/tools/file_read.rs`:

**Key features:**

- Direct file reads with optional line ranges
- Directory listings when a path resolves to a folder
- Binary detection to avoid noisy output
- Path-safety checks against repository boundaries

**Study this for**: File system operations, precise reads, structured output

### Code Search Tool

From `src/agents/tools/code_search.rs`:

**Key features:**

- Pattern matching across codebase
- Language-aware search
- Context around matches
- Result ranking

**Study this for**: Search implementations, regex patterns, result formatting

### Workspace Tool

From `src/agents/tools/workspace.rs`:

**Key features:**

- Stateful note-taking
- Task management
- Multiple action types
- Internal state synchronization

**Study this for**: Stateful tools, action-based interfaces, concurrent access

### Repo Map Tool

From `src/agents/tools/repo_map.rs`:

**Key features:**

- Non-`String` `Output` — uses `type Output = RepoMap;` (a `Serialize`/`Deserialize` struct)
- Re-exports both the tool struct (`RepoMapTool`) and its args type (`RepoMapArgs`) from `mod.rs`
- Walks the repo with `ignore::WalkBuilder`, scores files, and respects a token budget
- Extracts top-level definitions and imports across many languages with `regex` patterns
- Tests live in `src/agents/tools/tests/repo_map_tests.rs`

**Study this for**: Struct outputs, language-agnostic source scanning, budget-aware truncation.

### Git Blame Tool

From `src/agents/tools/git.rs` (the `GitBlame` struct):

**Key features:**

- Line-range blame plus recent file commits
- Uses the same `get_current_repo()` helper as other git tools
- Tests live in `src/agents/tools/tests/git_blame_tests.rs`

**Study this for**: Line-range arguments, defaulting via `#[serde(default)]`, and exercising `git` end-to-end in tests with a temporary repository.

### Git Show Tool

From `src/agents/tools/git.rs` (the `GitShow` struct):

**Key features:**

- Inspects a specific commit (metadata + diff) by revision
- Caps output size to stay inside the LLM context budget
- Tests live in `src/agents/tools/tests/git_show_tests.rs`

**Study this for**: Commit-level introspection and clamping output for context-budget safety.

### Static Analysis Tool

From `src/agents/tools/static_analysis.rs`:

**Key features:**

- Runs installed linters asynchronously via `tokio::process::Command`
- Times each command out with `tokio::time::timeout` and truncates noisy output
- Exposes a `JsonSchema` enum (`StaticAnalyzer` with `Auto`, `Rust`, `Python`, `Javascript`, `Go`) as an argument
- Registers three names from `mod.rs`: `StaticAnalysis` (the tool), `StaticAnalysisArgs` (the args), and `StaticAnalyzer` (the analyzer enum)
- Tests live in `src/agents/tools/tests/static_analysis_tests.rs`

**Study this for**: Async subprocess tools, enum-typed arguments, and testing complex selection logic by injecting an availability oracle.

### Content Update Tools (Studio Chat)

From `src/agents/tools/content_update.rs`:

`UpdateCommitTool`, `UpdatePRTool`, and `UpdateReviewTool` are wired into Studio's chat capability so Iris can mutate the displayed commit/PR/review directly while chatting. Each tool holds a cloned `ContentUpdateSender` (an `mpsc::Sender<ContentUpdate>`) and forwards a `ContentUpdate::Commit`/`PR`/`Review` over the channel. The Studio app side instantiates the receiver via `create_content_update_channel()` and dispatches the resulting updates into mode state.

**Study this for**: Tools that bridge the agent loop back into UI state, and the channel-based pattern for streaming structured side effects to the host.

### Tool struct vs args naming pattern

Most tools follow the `(Tool, ToolArgs)` pair convention. A few tools register a third name — typically an enum used inside the args:

```rust
// src/agents/tools/mod.rs
pub use repo_map::{RepoMap, RepoMapArgs, RepoMapTool};
pub use static_analysis::{StaticAnalysis, StaticAnalysisArgs, StaticAnalyzer};
```

When your tool's args reference a public enum or sub-struct the LLM is expected to fill in, re-export it alongside the tool so it's reachable from `crate::agents::tools::*`. The capability TOML can then cite the enum variants by name in workflow guidance.

## Common Tool Helpers

Use the shared utilities in `src/agents/tools/common.rs`:

```rust
use super::common::{get_current_repo, parameters_schema};

// Get current Git repository
let repo = get_current_repo()?;

// Generate JSON schema for args
let params = parameters_schema::<MyToolArgs>();
```

## Error Type Macro

Use the standard error macro:

```rust
// At top of your tool file
crate::define_tool_error!(MyToolError);

// Now you can use MyToolError(String) in your tool
```

This creates a consistent error type that works with the `Tool` trait.

## Debugging Tools

Test tool execution with debug mode:

```bash
just gen-debug
```

This shows:

- Which tools Iris calls
- Arguments passed to each tool
- Tool output
- Iris's reasoning about the results

## Integration with Capabilities

Reference your tool in capability TOML files:

```toml
task_prompt = """
## Tools Available
- `dependency_analyzer(manifest_type, include_dev)` - Analyze project dependencies
- `git_diff()` - Get code changes
- `file_read(path="...")` - Analyze specific files

## Workflow
1. Use `dependency_analyzer()` to understand project tech stack
2. Then analyze relevant source files with `file_read()`
"""
```

## Next Steps

- **Create capabilities** that use your tool → [Adding Capabilities](./capabilities.md)
- **Add Studio modes** to surface tool data → [Adding Studio Modes](./modes.md)
- **Contribute** your tool back → [Contributing](./contributing.md)
