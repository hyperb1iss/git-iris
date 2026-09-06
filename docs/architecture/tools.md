# Tool System

Tools are functions that Iris calls to gather information. Built on [rig-core 0.37](https://docs.rs/rig-core/0.37.0) (imported as `rig`), they provide structured, type-safe interfaces for code analysis and Git operations.

**Location:** `src/agents/tools/`

## Design Philosophy

### Tools Provide Data, Not Decisions

A critical principle of Git-Iris:

- **Tools return structured information** (diffs, file contents, commit history)
- **Iris makes decisions** (what's important, how to describe changes)
- **No hardcoded heuristics** in tools for determining commit messages or review priorities

This ensures the LLM drives intelligence while tools stay focused on data access.

### Type-Safe Interfaces

All tools use Rig's `Tool` trait:

```rust
#[async_trait::async_trait]
pub trait Tool {
    const NAME: &'static str;
    type Error;
    type Args: JsonSchema + DeserializeOwned;
    type Output: Serialize;

    async fn definition(&self, _: String) -> ToolDefinition;
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error>;
}
```

**Benefits:**

- Automatic JSON schema generation for arguments
- Type-safe argument parsing
- Structured error handling
- Self-documenting tool definitions

## Core Tools

### Git Operations

#### `git_status`

**Purpose:** Get repository status

**Arguments:**

```rust
pub struct GitStatusArgs {
    pub include_unstaged: bool,  // Default: false
}
```

**Returns:** String with branch name and file list

**Example:**

```
Branch: main
Files changed: 3
  src/agents/iris.rs: Modified
  src/types/commit.rs: Added
  README.md: Modified
```

#### `git_diff`

**Purpose:** Get staged changes (or a range diff) with relevance scoring and semantic analysis

**Arguments:**

```rust
pub struct GitDiffArgs {
    pub detail: DetailLevel,             // summary (default) or standard
    pub from: Option<String>,            // For PR/review (e.g., "main"); omit for staged
    pub to: Option<String>,              // Defaults to HEAD when `from` is set
    pub files: Option<Vec<String>>,      // Filter to specific repo-relative paths
}

pub enum DetailLevel {
    Summary,   // File list with stats and relevance scores (default)
    Standard,  // Full diffs (use with `files` for targeted analysis on large changesets)
}
```

There are exactly two detail levels. The `files` filter is what large-changeset workflows use to pull full diffs for the highest-relevance paths without re-streaming everything.

**Returns:** Formatted diff with metadata

**Key features:**

- **Relevance scoring** (0.0-1.0) for each file
- **Semantic change detection** (function additions, type changes, refactors)
- **Size guidance** for context strategy (3 buckets: Small, Medium, Large; plus Filtered when `files` is supplied)
- **Sorted by relevance** (most important first)

**Example output:**

```
=== DIFF SUMMARY ===
Size: Medium (8 files, 347 lines changed)
Guidance: Focus on files with >60% relevance (top 5 shown)

=== CHANGES (sorted by relevance) ===

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📄 src/agents/iris.rs [MODIFIED] ★★★★★ 95% relevance
   Reasons: source code, core source, substantive changes, adds function
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

@@ -310,6 +310,15 @@
 pub struct IrisAgent {
+    /// Fast model for subagents
+    fast_model: Option<String>,
}
...
```

**Relevance scoring** considers:

- Change type (added > modified > deleted)
- File type (source > config > docs)
- Path patterns (src/ > test/)
- Diff size (substantive preferred over trivial or massive)
- Semantic patterns (function defs, imports, types)

See [Context Strategy](./context.md) for the algorithm.

#### `git_log`

**Purpose:** Fetch recent commits, or commits in a range, for style reference or PR/release context

**Arguments:**

```rust
pub struct GitLogArgs {
    pub count: Option<usize>,   // Number of recent commits (default: 10)
    pub from: Option<String>,   // Start of range (commit/branch). When set, requires `to`.
    pub to: Option<String>,     // End of range; defaults to "HEAD" when only `from` is set.
}
```

When `from` is provided, the tool returns commits in `from..to` and appends a deduplicated contributor list (bot accounts filtered out). Otherwise it returns the N most recent commits.

**Returns:** Recent commit messages

```
✨ Add parallel analysis for large changesets
♻️ Refactor agent builder for Send safety
📝 Update documentation with architecture diagrams
```

Iris uses this to match project style and to assemble release notes.

#### `git_show`

**Purpose:** Inspect a historical commit's message, metadata, stat, and patch.

**Arguments:**

```rust
pub struct GitShowArgs {
    pub commit: String,                  // commit hash, tag, or branch name
    pub files: Option<Vec<PathBuf>>,     // optional repo-relative paths to filter the patch
    pub max_output_chars: usize,         // default 20000, clamped to 1000..=50000
}
```

`commit` is validated against `git rev-parse --verify --quiet <commit>^{commit}`; whitespace or leading `-` is rejected to keep the shell-out safe. Use this after `git_log` or `git_blame` when a historical commit's exact patch would clarify intent, prior behavior, or regression risk.

**Returns:** Formatted commit metadata, stat, and patch — truncated to `max_output_chars`.

#### `git_changed_files`

**Purpose:** Get list of changed files (no diffs)

**Arguments:**

```rust
pub struct GitChangedFilesArgs {
    pub from: Option<String>,   // start of range; with `to`, lists files in `from..to`
    pub to: Option<String>,     // end of range; with only `to`, lists files in that single commit
}
```

When both `from` and `to` are omitted, the tool returns staged files. `from` without `to` is rejected — file listing requires a complete range.

**Returns:** Simple file list

```
src/agents/iris.rs (Modified)
src/types/commit.rs (Added)
README.md (Modified)
```

Useful for quick changeset overview.

#### `git_blame`

**Purpose:** Line-level blame for a file range plus the recent commits that touched the file.

**Arguments:**

```rust
pub struct GitBlameArgs {
    pub file: PathBuf,                // repo-relative path
    pub start_line: u32,              // 1-based, defaults to 1
    pub end_line: Option<u32>,        // defaults to `start_line`
    pub recent_commits: usize,        // default 3, clamped 1..=10
}
```

Use this when ownership, prior intent, or stylistic precedent would sharpen commit messages, PR descriptions, or semantic explanations. The output combines the requested line range, blame metadata for each line, and the most recent N commits touching the file.

### File Operations

#### `file_read`

**Purpose:** Read file contents directly

**Arguments:**

```rust
pub struct FileReadArgs {
    pub path: String,              // File path
    pub start_line: Option<usize>, // Optional: line to start from
    pub num_lines: Option<usize>,  // Optional: number of lines
}
```

**Returns:** File contents with optional range

**Example:**

```rust
// Read entire file
file_read({ "path": "src/agents/iris.rs" })

// Read specific range
file_read({
    "path": "src/agents/iris.rs",
    "start_line": 100,
    "num_lines": 50
})
```

**When to use:**

- Reading project configuration (Cargo.toml, package.json)
- Examining specific functions or modules
- Understanding context not visible in diffs

#### `code_search`

**Purpose:** ripgrep-backed search for patterns, symbols, or text across files

**Arguments:**

```rust
pub struct CodeSearchArgs {
    pub query: String,                 // Function name, class name, variable, text, or pattern
    pub search_type: SearchType,       // function | class | variable | text (default) | pattern
    pub file_pattern: Option<String>,  // Optional file glob (e.g., "*.rs")
    pub max_results: usize,            // Default 20, capped at 100
}

pub enum SearchType {
    Function,  // function/method definitions
    Class,     // class/struct/enum definitions
    Variable,  // variable assignments
    Text,      // case-insensitive text search (default)
    Pattern,   // regex pattern
}
```

**Returns:** JSON object with `query`, `search_type`, `results: Vec<SearchResult>`, `total_found`, `max_results`.

**Examples:**

```jsonc
// Find function definitions named "execute_with_agent"
{ "query": "execute_with_agent", "search_type": "function", "file_pattern": "*.rs" }

// Find usages of a string across the repo
{ "query": "StructuredResponse::Review", "search_type": "text", "max_results": 50 }
```

**Best practices:**

- Use sparingly — `file_read` is better for known files, and `repo_map` is better for cross-file orientation.
- Prefer the targeted `search_type` variants (`function`/`class`/`variable`) over freeform text when you know what you're looking for.
- Use `file_pattern` to scope ripgrep at the input layer.

### Repository Orientation

#### `repo_map`

**Purpose:** Build a compact, ranked map of source files, their definitions, imports, and changed-or-mentioned-file signals — the codebase skeleton without reading every file.

**Arguments:**

```rust
pub struct RepoMapArgs {
    pub token_budget: u32,             // default 2000, max 8000
    pub mentioned_files: Vec<PathBuf>, // files to boost in ranking
    pub max_files: usize,              // default 60, clamped 1..=200
}
```

**Returns:** A `RepoMap` struct (not a string):

```rust
pub struct RepoMap {
    pub files_analyzed: usize,
    pub files_shown: usize,
    pub changed_files: Vec<PathBuf>,
    pub mentioned_files: Vec<PathBuf>,
    pub content: String,   // rendered ranked map within `token_budget`
}
```

`repo_map` walks the repository through `WalkBuilder` (honors `.gitignore`), extracts up to 12 definition matches and 6 import matches per file using language-aware regex sets (Rust, TypeScript/JavaScript, Python, Go, Kotlin/Swift, Ruby, Lua, shell), ranks each file by definitions/imports plus changed-status and mentioned-status boosts, and renders the top-N within `token_budget`. Use it for broad cross-file orientation before targeted reads.

### Static Analysis

#### `static_analysis`

**Purpose:** Run installed linters directly so review-quality evidence comes from a real analyzer, not a manual eyeball.

**Arguments:**

```rust
pub struct StaticAnalysisArgs {
    pub analyzer: StaticAnalyzer,   // auto (default) | rust | python | javascript | go
    pub timeout_secs: u64,          // default 300, clamped 1..=600
    pub max_output_chars: usize,    // default 12000, clamped 512..=40000
}
```

`auto` selects analyzers by detecting project files (`Cargo.toml`, `pyproject.toml`/`ruff.toml`/`setup.cfg`, `package.json`, Go modules). The tool only runs commands that are installed on `PATH`:

| Language                | Command sequence                                            |
| ----------------------- | ----------------------------------------------------------- |
| Rust                    | `cargo clippy --workspace --no-deps --message-format short` |
| Python                  | `ruff check .`                                              |
| JavaScript / TypeScript | `biome check .` if installed, otherwise `oxlint .`          |
| Go                      | `golangci-lint run` if installed, otherwise `go vet ./...`  |

Output is truncated to `max_output_chars` per command and prefixed with a one-line reason explaining why the command was selected. When no installed analyzer matches, the tool returns an availability summary instead of failing. Because these analyzers can execute project build scripts, plugins, or configuration, run them only in trusted workspaces.

**Used by:** the `review` capability prefers analyzer findings over speculative manual notes when both are available.

### Project Documentation

#### `project_docs`

**Purpose:** Read project documentation and conventions

**Arguments:**

```rust
pub struct ProjectDocsArgs {
    pub doc_type: DocType,   // enum, default: readme
    pub max_chars: usize,    // default: 20000
}

pub enum DocType {
    Readme,        // README.md / README.rst / README.txt (default)
    Contributing, // CONTRIBUTING.md
    Changelog,    // CHANGELOG.md, HISTORY.md
    License,      // LICENSE files
    CodeOfConduct, // CODE_OF_CONDUCT.md
    Agents,       // AGENTS.md, CLAUDE.md, .github/copilot-instructions.md, ...
    Context,      // concise README + agent-instructions summary (shared budget)
    All,          // all supported project docs
}
```

- `context` — A concise README + agent-instructions summary

**Returns:** Document contents (truncated when total chars exceed `max_chars`).

**Example:**

```jsonc
// Get a compact project context snapshot
{ "doc_type": "context", "max_chars": 8000 }
```

**Why this matters:**

Every project has conventions (commit style, terminology, architecture patterns). Iris can grab a compact context snapshot quickly with `doc_type: "context"`, then request targeted docs when she needs the full file. The `max_chars` budget is enforced per file, with `context` treating it as a shared budget across the snapshot.

### Repository Metadata

#### `git_repo_info`

**Purpose:** Get repository metadata

**Arguments:** None

**Returns:** JSON with repo details

**Example:**

```json
{
  "path": "/Users/user/git-iris",
  "branch": "main",
  "remote": "https://github.com/user/git-iris",
  "commit_count": 342
}
```

Useful for including repo URLs in PR descriptions or release notes.

### Agent Delegation

#### `workspace`

**Purpose:** Iris's persistent notes and task tracking (main agent only — not attached to subagents).

**Arguments:**

```rust
pub struct WorkspaceArgs {
    pub action: WorkspaceAction,        // add_note | add_task | update_task | get_summary (default)
    pub content: Option<String>,        // note text or task description
    pub priority: Option<TaskPriority>, // low | medium (default) | high | critical
    pub task_index: Option<usize>,      // 0-based; required for update_task
    pub status: Option<TaskStatus>,     // pending (default) | in_progress | completed | blocked
}
```

**Returns:** Current workspace state (notes + tasks summary).

**Examples:**

```jsonc
// Add a note
{ "action": "add_note", "content": "Auth changes affect 3 modules" }

// Add a task
{ "action": "add_task", "content": "Verify migration", "priority": "high" }

// Update a task's status
{ "action": "update_task", "task_index": 0, "status": "completed" }

// Get current workspace summary (default)
{ "action": "get_summary" }
```

**Use case:** Iris tracks findings across many tool calls, building up context and a TODO list before generating the final output. The workspace is per-agent-instance and is reset whenever a fresh agent is built.

#### `parallel_analyze`

**Purpose:** Spawn concurrent subagents for large tasks (main agent only).

**Arguments:**

```rust
pub struct ParallelAnalyzeArgs {
    pub tasks: Vec<String>,         // 1..=10 focused prompts (JSON schema enforces minItems/maxItems)
    pub max_turns: Option<usize>,   // optional per-subagent turn budget; clamped 1..=100
}
```

If `max_turns` is omitted the subagents inherit the budgets configured on `ParallelAnalyze::with_limits` — which come from `Config.subagent_max_turns` (default 20) and `Config.subagent_timeout_secs` (default 120). Increase `max_turns` for repository-wide sweeps; lower it to cap cost or runaway tool loops.

**Returns:** Aggregated results

**Example:**

```jsonc
{
  "tasks": [
    "Analyze authentication changes in src/auth/",
    "Review API endpoint changes in src/api/",
    "Check database migration in migrations/",
  ],
  "max_turns": 30,
}
```

**Returns:**

```json
{
  "results": [
    {
      "task": "Analyze authentication changes...",
      "result": "The auth module adds OAuth2 support...",
      "success": true
    },
    {
      "task": "Review API endpoint changes...",
      "result": "Three new endpoints added for user management...",
      "success": true
    }
  ],
  "successful": 2,
  "failed": 0,
  "execution_time_ms": 3421
}
```

**How it works:**

1. Spawns N independent subagents (using fast model)
2. Each subagent has core tools (`git_diff`, `file_read`, etc.)
3. Runs concurrently with separate context windows
4. Main agent synthesizes results

**When to use:**

- Changesets >20 files or >1000 lines
- Batch commit analysis
- Multi-module refactors

See [Context Strategy](./context.md) for decision criteria.

#### `analyze_subagent`

**Purpose:** Delegate a single focused task to a sub-agent

**Arguments:** Free-form prompt string

**Returns:** Sub-agent's analysis

**Example:**

```rust
analyze_subagent("Analyze the security implications of changes in src/auth/oauth.rs")
```

**Difference from `parallel_analyze`:**

- Single task vs. multiple concurrent tasks
- Simpler interface
- Use for deep dives on specific files/modules

### Content Update Tools (Studio Only)

These tools are only available in Studio chat mode:

#### `update_commit`

Update the current commit message in Studio.

#### `update_pr`

Update the current PR description in Studio.

#### `update_review`

Update the current review content in Studio.

**Example Studio interaction:**

```
User: "Make the commit message more concise"
Iris: [Calls update_commit with revised message]
```

## Tool Registry

To ensure consistency between main agents and subagents, Git-Iris uses a **tool registry macro** that wires the eleven core tools onto any agent builder.

**Source:** `src/agents/tools/registry.rs`

```rust
#[macro_export]
macro_rules! attach_core_tools {
    ($builder:expr) => {{
        use $crate::agents::debug_tool::DebugTool;
        use $crate::agents::tools::{
            CodeSearch, FileRead, GitBlame, GitChangedFiles, GitDiff, GitLog, GitShow, GitStatus,
            ProjectDocs, RepoMapTool, StaticAnalysis,
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
    }};
}

pub const CORE_TOOLS: &[&str] = &[
    "git_status", "git_diff", "git_log", "git_show",
    "git_changed_files", "git_blame",
    "file_read", "code_search",
    "repo_map", "static_analysis", "project_docs",
];
```

A `#[test]` asserts `CORE_TOOLS.len() == 11` so drift between the macro and the constant trips CI immediately.

**Usage:**

```rust
// Main agent
let agent = attach_core_tools!(builder)
    .tool(DebugTool::new(GitRepoInfo))   // Main agent only
    .tool(DebugTool::new(self.workspace.clone())) // Main agent only
    .tool(DebugTool::new(ParallelAnalyze::with_limits(/* … */)?)) // Main agent only
    .tool(sub_agent)                     // analyze_subagent (Rig agent-as-tool)
    .build();

// Subagent (no delegation tools — prevents recursion)
let sub_agent = attach_core_tools!(sub_builder).build();
```

**Benefits:**

- Subagents always have the same eleven analysis tools as the main agent.
- Changes to the core tool set apply everywhere through one macro.
- No drift between agent implementations — enforced by the `CORE_TOOLS` count test.

## Creating a Custom Tool

### Step 1: Define Arguments and Output

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MyToolArgs {
    pub input: String,
    #[serde(default)]
    pub optional_flag: bool,
}

#[derive(Debug, Serialize)]
pub struct MyToolOutput {
    pub result: String,
    pub metadata: HashMap<String, String>,
}
```

### Step 2: Implement the Tool

```rust
use rig::tool::Tool;
use rig::completion::ToolDefinition;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyTool;

impl Tool for MyTool {
    const NAME: &'static str = "my_tool";
    type Error = anyhow::Error;
    type Args = MyToolArgs;
    type Output = MyToolOutput;

    async fn definition(&self, _: String) -> ToolDefinition {
        ToolDefinition {
            name: "my_tool".to_string(),
            description: "What this tool does and when to use it".to_string(),
            parameters: crate::agents::tools::parameters_schema::<MyToolArgs>(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output> {
        // Implement tool logic
        let result = format!("Processed: {}", args.input);

        Ok(MyToolOutput {
            result,
            metadata: HashMap::new(),
        })
    }
}
```

### Step 3: Add to Agent

```rust
let agent = client.agent(model)
    .tool(DebugTool::new(MyTool))
    .build();
```

### Step 4: Test

```bash
cargo test
```

## Tool Design Best Practices

### 1. Single Responsibility

Each tool should do **one thing well**:

✅ **Good:** `git_diff` — Returns diffs with metadata
❌ **Bad:** `analyze_and_generate_commit` — Mixes analysis and generation

### 2. Structured Output

Return structured data, not formatted text:

✅ **Good:**

```rust
pub struct DiffOutput {
    pub files: Vec<FileChange>,
    pub total_lines: usize,
    pub size_category: String,
}
```

❌ **Bad:**

```rust
pub struct DiffOutput {
    pub formatted_text: String,  // Unstructured
}
```

### 3. Clear Descriptions

Tool descriptions should explain:

- **What** the tool does
- **When** to use it
- **What** it returns

Example:

```rust
description: "Get staged changes with relevance scores. Use this to see what's \
              changed and prioritize files for analysis. Returns diffs sorted \
              by importance with semantic change detection."
```

### 4. Sensible Defaults

Make common use cases simple:

```rust
#[derive(JsonSchema, Default)]
pub struct GitDiffArgs {
    #[serde(default)]
    pub detail: DetailLevel,  // Defaults to Summary
}
```

### 5. Error Context

Provide helpful error messages:

```rust
Err(anyhow::anyhow!(
    "Failed to read file '{}': {}. Make sure the path is relative to repo root.",
    path,
    e
))
```

## Debug Wrapper

All tools are wrapped in `DebugTool` for instrumentation:

```rust
pub struct DebugTool<T> {
    inner: T,
}

impl<T: Tool> Tool for DebugTool<T> {
    async fn call(&self, args: Self::Args) -> Result<Self::Output> {
        debug::debug_tool_call(Self::NAME, &args);
        let timer = debug::DebugTimer::start(format!("Tool: {}", Self::NAME));

        let result = self.inner.call(args).await;

        timer.finish();
        if result.is_ok() {
            debug::debug_tool_success(Self::NAME);
        } else {
            debug::debug_tool_error(Self::NAME, &format!("{:?}", result));
        }

        result
    }
}
```

Enable with `--debug` for color-coded tool execution traces.

## Testing Tools

### Unit Tests

Test tool logic directly:

```rust
#[tokio::test]
async fn test_git_diff() {
    let tool = GitDiff;
    let args = GitDiffArgs {
        detail: DetailLevel::Summary,
        from: None,
        to: None,
        files: None,
    };

    let result = tool.call(args).await.unwrap();
    assert!(result.contains("CHANGES SUMMARY"));
}
```

### Integration Tests

Test tools within agent context:

```rust
#[tokio::test]
async fn agent_uses_git_diff() {
    let agent = IrisAgent::new("openai", "gpt-6-astra").unwrap();
    let response = agent.execute_task("commit", "Generate message").await.unwrap();

    // Verify the agent called git_diff and produced output
    assert!(matches!(response, StructuredResponse::CommitMessage(_)));
}
```

## Common Patterns

### Pagination

For large results:

```rust
pub struct SearchArgs {
    pub pattern: String,
    pub max_results: usize,  // Default: 50
    pub offset: usize,       // Default: 0
}
```

### Context Windows

For reading large files:

```rust
pub struct FileReadArgs {
    pub path: String,
    pub start_line: Option<usize>,
    pub num_lines: Option<usize>,  // Default: entire file
}
```

### Progressive Detail

Offer a default-light detail level, then a heavier one that the caller can scope. `git_diff` uses this pattern:

```rust
pub enum DetailLevel {
    Summary,   // file list + relevance scores; default
    Standard,  // full diffs (combine with `files: Vec<String>` for targeted analysis)
}
```

Iris starts with `Summary`, reads the size guidance, then calls `Standard` with a `files` filter scoped to the highest-relevance paths.

## Performance Considerations

### Lazy Evaluation

Compute expensive operations only when needed:

```rust
// ✅ Good: Compute relevance only if the caller asked for diffs
if matches!(args.detail, DetailLevel::Standard) {
    calculate_relevance_scores(&files);
}
```

### Caching

Tools can cache expensive results:

```rust
static REPO_INFO_CACHE: OnceCell<RepoInfo> = OnceCell::new();

async fn call(&self, _: Args) -> Result<Output> {
    let info = REPO_INFO_CACHE.get_or_try_init(|| {
        expensive_repo_scan()
    })?;
    Ok(info.clone())
}
```

### Parallel Execution

Tools can use concurrency internally:

```rust
let results = futures::future::join_all(
    files.iter().map(|f| analyze_file(f))
).await;
```

## Next Steps

- [Capabilities](./capabilities.md) — How tools are used in task prompts
- [Agent System](./agent.md) — How agents call tools
- [Context Strategy](./context.md) — Relevance scoring algorithm
