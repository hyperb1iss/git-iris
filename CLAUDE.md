# Git-Iris Developer Guide

> **Note:** This is a quick-reference guide for AI assistants. For comprehensive documentation, see:
>
> - **[Architecture Documentation](docs/architecture/)** — System design, patterns, and data flow
> - **[Theme System Documentation](docs/themes/)** — SilkCircuit design language and theming
> - **[Studio Internals](docs/studio-internals/)** — Deep dive into TUI implementation
> - **[Extension Guide](docs/extending/)** — Creating new capabilities, tools, and modes

## Architecture Overview

Git-Iris uses an agent-first architecture powered by **Iris**, an LLM-driven agent built on the [Rig framework](https://docs.rs/rig/0.42.0). Iris dynamically explores codebases using tool calls rather than dumping all context upfront.

### Core Principles

- **LLM-First**: The LLM makes all intelligent decisions—we avoid deterministic heuristics
- **Tool-Based Context**: Iris gathers precisely what she needs via tool calls
- **Unified Interface**: Studio provides a single TUI for all capabilities
- **Event-Driven State**: Reducer-centric event flow for predictable, testable state management

## Project Structure

```
src/
├── agents/                       # Agent framework (core of Git-Iris)
│   ├── iris.rs                   # Main agent implementation
│   ├── core.rs                   # Backend abstraction (OpenAI/Anthropic/Google)
│   ├── context.rs                # TaskContext and shared agent context
│   ├── provider.rs               # Provider-specific agent builders (caching, models)
│   ├── setup.rs                  # IrisAgentService entry point
│   ├── status.rs                 # Real-time status tracking
│   ├── status_messages.rs        # Witty status messages via fast model
│   ├── debug.rs                  # Debug mode output formatting
│   ├── debug_tool.rs             # Tool wrapper for debug instrumentation
│   ├── output_validator.rs       # JSON recovery for malformed responses
│   ├── capabilities/             # Task-specific prompts (TOML, 8 total)
│   │   ├── commit.toml           # Commit message generation
│   │   ├── review.toml           # Structured code review (findings, severity)
│   │   ├── pr.toml               # PR description generation
│   │   ├── changelog.toml        # Changelog generation
│   │   ├── release_notes.toml    # Release notes
│   │   ├── chat.toml             # Interactive chat capability
│   │   ├── semantic_blame.toml   # "Why does this code exist?"
│   │   └── verify.toml           # Critic verification pass (internal)
│   └── tools/                    # Tools Iris can use
│       ├── registry.rs           # CORE_TOOLS list + attach_core_tools! macro
│       ├── common.rs             # Shared tool utilities (repo root, schemas)
│       ├── git.rs                # git_diff, git_log, git_status, git_show,
│       │                         # git_changed_files, git_blame, git_repo_info
│       ├── file_read.rs          # File content reading and targeted excerpts
│       ├── code_search.rs        # Pattern searching
│       ├── repo_map.rs           # Ranked codebase orientation map
│       ├── static_analysis.rs    # Direct linter runs (rust/python/js/go)
│       ├── docs.rs               # Project documentation (README, CLAUDE.md)
│       ├── workspace.rs          # Iris's notes and task tracking
│       ├── parallel_analyze.rs   # Concurrent subagent processing
│       └── content_update.rs     # Chat tools that update commit/PR/review
│
├── studio/                       # Iris Studio TUI (Ratatui-based)
│   ├── app/                      # Main event loop and app coordination
│   │   ├── mod.rs                # StudioApp lifecycle
│   │   └── agent_tasks.rs        # Agent task spawning and orchestration
│   ├── state/                    # Centralized state for all modes
│   │   ├── mod.rs                # StudioState root and helpers
│   │   ├── chat.rs               # ChatState (messages, streaming, tools)
│   │   └── modes.rs              # ExploreState, CommitState, ReviewState, etc.
│   ├── reducer/                  # Reducer-centric state transitions
│   │   ├── mod.rs                # Top-level reduce() dispatch
│   │   ├── agent.rs              # Agent task events
│   │   ├── content.rs            # Content updates (commit, PR, review)
│   │   ├── git.rs                # Git operation events
│   │   ├── modal.rs              # Modal open/close transitions
│   │   ├── navigation.rs         # Mode switching and panel focus
│   │   ├── settings.rs           # Settings modal state
│   │   └── ui.rs                 # UI-level events (resize, scroll)
│   ├── events.rs                 # StudioEvent, AgentTask, SideEffect enums
│   ├── history.rs                # Audit trail and session persistence
│   ├── layout.rs                 # Layout calculations and panel sizing
│   ├── theme.rs                  # Studio-specific style derivation
│   ├── utils.rs                  # Shared rendering helpers
│   ├── components/               # Reusable UI components
│   │   ├── code_view.rs          # Syntax-highlighted source display
│   │   ├── diff_view.rs          # Unified diff rendering with hunks
│   │   ├── file_tree.rs          # Directory navigation with git status
│   │   ├── message_editor.rs     # Text editing with cursor management
│   │   └── syntax.rs             # Syntect-based syntax highlighting
│   ├── render/                   # Mode-specific rendering
│   │   ├── commit.rs             # Commit mode panels
│   │   ├── explore.rs            # Explore mode panels
│   │   ├── review.rs             # Review mode panels
│   │   ├── pr.rs                 # PR mode panels
│   │   ├── changelog.rs          # Changelog mode panels
│   │   ├── release_notes.rs      # Release notes panels
│   │   ├── chat.rs               # Chat panel with markdown
│   │   └── modals/               # Modal renderers (12 files: help, settings,
│   │                             # search, theme/preset/ref/emoji/commit_count
│   │                             # selectors, chat_modal, confirm, instructions)
│   └── handlers/                 # Input handling
│       ├── mod.rs                # Cross-mode key dispatch and global bindings
│       ├── commit.rs             # Commit mode handlers
│       ├── explore.rs            # Explore mode handlers
│       ├── review.rs             # Review mode handlers
│       ├── pr.rs                 # PR mode handlers
│       ├── changelog.rs          # Changelog mode handlers
│       ├── release_notes.rs      # Release notes mode handlers
│       └── modals/               # Modal input handlers (10 files matching
│                                 # the modal renderers)
│
├── companion/                    # Iris Companion (ambient session awareness)
│   ├── session.rs                # SessionState and FileActivity tracking
│   ├── branch_memory.rs          # Per-branch focus and memory persistence
│   ├── storage.rs                # Persistence backend
│   └── watcher.rs                # Live file watching via notify
│
├── types/                        # Response type definitions
│   ├── commit.rs                 # GeneratedMessage (emoji/title/message/completion)
│   ├── pr.rs                     # MarkdownPullRequest
│   ├── review.rs                 # Review (structured findings + metadata + stats)
│   ├── changelog.rs              # MarkdownChangelog
│   └── release_notes.rs          # MarkdownReleaseNotes
│
├── services/                     # Pure operations (no LLM)
│   └── git_commit.rs             # GitCommitService for git operations
│
├── git/                          # Git2 wrapper module
├── github/                       # GitHub API client (octocrab wrapper)
├── config/                       # Configuration loading helpers
├── cli.rs                        # CLI entry point
├── commands.rs                   # Command handlers
├── common.rs                     # Shared CLI params (CommonParams, --critic)
├── providers.rs                  # LLM provider configuration
├── config.rs                     # Configuration management
├── theme.rs                      # Opaline-backed theme engine (re-exports)
├── github.rs                     # GitHub publishing (PR descriptions, reviews)
├── crypto.rs                     # Process-wide rustls/aws-lc-rs provider pin
├── gitmoji.rs                    # Emoji processing
└── output.rs                     # Git output formatting
```

## Iris Studio Architecture

Studio is built around a **reducer-centric event loop** for predictable state management:

```
┌─────────────────────────────────────────────────────────────┐
│                        Studio App                           │
├─────────────────────────────────────────────────────────────┤
│  Input Events (keyboard, mouse)                             │
│         │                                                   │
│         ▼                                                   │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────┐ │
│  │   Handler   │ -> │   Reducer   │ -> │  Side Effects   │ │
│  │ (map input  │    │ (state core)│    │ (spawn agent,   │ │
│  │  to event)  │    │             │    │  load data)     │ │
│  └─────────────┘    └─────────────┘    └─────────────────┘ │
│                            │                                │
│                            ▼                                │
│                     ┌─────────────┐                         │
│                     │    State    │                         │
│                     │  (updated)  │                         │
│                     └─────────────┘                         │
│                            │                                │
│                            ▼                                │
│                     ┌─────────────┐                         │
│                     │   Render    │                         │
│                     │ (to frame)  │                         │
│                     └─────────────┘                         │
└─────────────────────────────────────────────────────────────┘
```

### Key Components

**Events (`events.rs`):**

- `StudioEvent` enum captures all possible state transitions
- Events are dispatched from handlers and async tasks
- Clear, traceable data flow

**Reducer (`reducer/`):**

- Central event reducer split into submodules: `agent`, `content`, `git`, `modal`, `navigation`, `settings`, `ui`
- Avoids direct I/O by returning side effects as data
- Works alongside handlers and `StudioApp`, which still perform some direct UI/data updates

**Side Effects (`SideEffect` enum in `events.rs`):**

- `SpawnAgent { task }` — Start async agent execution
- `LoadData { data_type, from_ref, to_ref }` — Load git data asynchronously
- `ExecuteCommit { message }` / `ExecuteAmend { message }` — Perform git commit
- `GitStage(path)` / `GitUnstage(path)` / `GitStageAll` / `GitUnstageAll` — Index updates
- `GatherBlameAndSpawnAgent { file, start_line, end_line }` — Semantic blame flow
- `CopyToClipboard(text)`, `SaveSettings`, `RefreshGitStatus`, `LoadFileLog`, `LoadGlobalLog`, `Quit`

### Studio Modes

| Mode              | Description                        | State Struct        |
| ----------------- | ---------------------------------- | ------------------- |
| **Explore**       | Navigate codebase with AI insights | `ExploreState`      |
| **Commit**        | Generate/edit commit messages      | `CommitState`       |
| **Review**        | AI-powered code reviews            | `ReviewState`       |
| **PR**            | Pull request descriptions          | `PrState`           |
| **Changelog**     | Structured changelog generation    | `ChangelogState`    |
| **Release Notes** | Release documentation              | `ReleaseNotesState` |

### Chat Integration

Press `/` in any mode to open chat with Iris:

```rust
// Chat state tracks conversation (src/studio/state/chat.rs)
pub struct ChatState {
    pub messages: VecDeque<ChatMessage>,
    pub input: String,
    pub scroll_offset: usize,
    pub is_responding: bool,
    pub streaming_response: Option<String>,
    pub auto_scroll: bool,
    pub current_tool: Option<String>,
    pub tool_history: VecDeque<String>,
    pub error: Option<String>,
}
```

Iris can update content directly through tools:

- `update_commit` — Modify commit message
- `update_pr` — Modify PR description
- `update_review` — Modify review content

## Agent Architecture

### Capabilities

Each capability is defined in `src/agents/capabilities/*.toml`:

| Capability       | Output Type            | Description                                                                     |
| ---------------- | ---------------------- | ------------------------------------------------------------------------------- |
| `commit`         | `GeneratedMessage`     | Commit messages with emoji/title/body                                           |
| `review`         | `Review`               | Structured code review with findings, severity, confidence, GitHub inline links |
| `pr`             | `MarkdownPullRequest`  | Pull request descriptions                                                       |
| `changelog`      | `MarkdownChangelog`    | Keep a Changelog format                                                         |
| `release_notes`  | `MarkdownReleaseNotes` | Release documentation                                                           |
| `chat`           | Varies                 | Interactive conversation                                                        |
| `semantic_blame` | `SemanticBlame`        | "Why does this code exist?" history-aware explanation                           |
| `verify`         | `Critique`             | Critic verification pass — internal, runs after generation                      |

### Tools Available to Iris

| Tool                                                        | Purpose                                                 |
| ----------------------------------------------------------- | ------------------------------------------------------- |
| `git_diff(detail, from, to, files)`                         | Get changes with relevance scores; optional file filter |
| `git_log(count, from, to)`                                  | Recent commit history for style reference               |
| `git_status()`                                              | Repository status                                       |
| `git_changed_files(from, to)`                               | List of changed files                                   |
| `git_show(commit, files, max_output_chars)`                 | Inspect a historical commit (truncated to budget)       |
| `git_blame(file, start_line, end_line, recent_commits)`     | Line history and recent file commits                    |
| `git_repo_info()`                                           | Branch, remote, default-base metadata                   |
| `file_read(path, start, end)`                               | Read targeted file content and excerpts                 |
| `code_search()`                                             | Search for patterns, functions, classes                 |
| `repo_map(token_budget, mentioned_files, max_files)`        | Ranked codebase orientation map                         |
| `static_analysis(analyzer, timeout_secs, max_output_chars)` | Run rust/python/javascript/go linters directly          |
| `project_docs(doc_type)`                                    | Read README, AGENTS.md, CLAUDE.md                       |
| `workspace()`                                               | Iris's notes and task tracking                          |
| `parallel_analyze()`                                        | Concurrent subagent processing (per-call `max_turns`)   |
| `update_commit()` / `update_pr()` / `update_review()`       | Chat: update generated content in place                 |

The four extraction tools — `repo_map`, `git_blame`, `git_show`, and `static_analysis` — are **core tools**, attached to every main agent and subagent via `attach_core_tools!` in `src/agents/tools/registry.rs` (`CORE_TOOLS`, 11 entries).

### Context Strategy

Iris adapts her approach based on changeset size:

| Scenario                             | Strategy                                             |
| ------------------------------------ | ---------------------------------------------------- |
| Small (≤3 files, <100 lines total)   | Full context for all files                           |
| Medium (≤10 files, <500 lines total) | Prioritize files with >60% relevance score           |
| Large (everything else)              | Summaries by default; `parallel_analyze` for breadth |

Thresholds live in `format_diff_output` in `src/agents/tools/git.rs`. Iris also gets per-call control via `parallel_analyze`'s `max_turns` and the configurable `subagent_timeout_secs` / `subagent_max_turns` budgets.

### Adding a New Capability

1. Create `src/agents/capabilities/new_capability.toml`:

```toml
name = "my_capability"
description = "What it does"
output_type = "MyOutputType"

task_prompt = """
Instructions for Iris...
"""
```

2. Add output type to `src/agents/iris.rs` `StructuredResponse` enum
3. Add match arm in `execute_output_type()` for the new output type (`src/agents/iris.rs`)
4. Wire the capability into `load_capability_config()` (`src/agents/iris.rs`) — capabilities are dispatched via a `match capability { "commit" => CAPABILITY_COMMIT, ... }` block on embedded TOML strings, not a HashMap
5. (Optional) Add Studio mode in `src/studio/state/mod.rs`

## Output Types

Iris produces structured responses (all in `src/types/`):

| Type                   | Format   | Description                                                                      |
| ---------------------- | -------- | -------------------------------------------------------------------------------- |
| `GeneratedMessage`     | JSON     | `{ emoji, title, message, completion_message }`                                  |
| `MarkdownPullRequest`  | Markdown | `{ content: String }`                                                            |
| `Review`               | JSON     | `{ summary, metadata, findings[], stats }` — structured findings with confidence |
| `MarkdownChangelog`    | Markdown | `{ content: String }`                                                            |
| `MarkdownReleaseNotes` | Markdown | `{ content: String }`                                                            |

The `Markdown*` types use a simple wrapper, letting the LLM drive format while capability TOMLs provide guidelines. `Review` is fully structured — findings carry severity, category, file/line citations, and a confidence score. Findings are gated at confidence ≥ 70 for terminal display and GitHub inline publishing.

## SilkCircuit Design Language

Git-Iris follows the **SilkCircuit Neon** color palette for a cohesive, electric aesthetic.

### Color Palette

| Color           | Hex       | RGB               | Usage                           |
| --------------- | --------- | ----------------- | ------------------------------- |
| Electric Purple | `#e135ff` | `(225, 53, 255)`  | Active modes, markers, emphasis |
| Neon Cyan       | `#80ffea` | `(128, 255, 234)` | Paths, interactions, focus      |
| Coral           | `#ff6ac1` | `(255, 106, 193)` | Hashes, numbers, constants      |
| Electric Yellow | `#f1fa8c` | `(241, 250, 140)` | Warnings, timestamps            |
| Success Green   | `#50fa7b` | `(80, 250, 123)`  | Success, confirmations          |
| Error Red       | `#ff6363` | `(255, 99, 99)`   | Errors, danger                  |

### Backgrounds

| Surface   | Hex       | Usage             |
| --------- | --------- | ----------------- |
| Base      | `#121218` | Main background   |
| Panel     | `#181820` | Individual panels |
| Highlight | `#2d283c` | Selections        |
| Code      | `#1e1e28` | Code blocks       |

### Implementation

```rust
use colored::Colorize;

// Success message
println!("{}", "✨ Commit created".truecolor(80, 250, 123));

// Error message
println!("{}", "Error: No staged changes".truecolor(255, 99, 99));

// Commit hash
println!("Commit: {}", hash.truecolor(255, 106, 193));
```

### Typography

- Monospace fonts: JetBrains Mono, Fira Code, SF Mono
- Unicode box-drawing: `─`, `━`, `│`, `┌`, `┐`, `└`, `┘`
- Braille spinners: `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`

## Development Commands

All common tasks are available via [just](https://github.com/casey/just). Run `just` to see all recipes.

```bash
# Build
just build                   # Debug build
just build-release           # Release build

# Quality
just check                   # Lint + test (full gate)
just lint                    # Format check + clippy
just clippy-pedantic         # Clippy with pedantic warnings
just fix                     # Auto-fix clippy + formatting
just fmt                     # Format code

# Test
just test                    # Run all tests
just test-verbose            # Tests with output
just test-one <name>         # Run a specific test

# Run
just run -- gen --debug      # Color-coded agent execution
just studio                  # Launch Studio TUI
just gen-debug               # Generate commit with debug output
just run-debug -- gen        # Verbose RUST_LOG=debug logging

# Docs
just docs-dev                # Start VitePress dev server
just docs-build              # Build VitePress site
just docs-fmt                # Format docs markdown

# Docker / Release
just docker-build            # Build Docker image
just aur-update <version>    # Update AUR package
just brew-update             # Update Homebrew formula
```

Raw cargo commands still work if you prefer them.

## Testing Conventions

**Tests go in separate files, not inline with source code.**

### Directory Structure

```
src/
├── module/
│   ├── mod.rs           # Module code (NO #[cfg(test)] mod tests inline)
│   ├── submodule.rs     # Submodule code
│   └── tests/           # Test directory
│       ├── mod.rs       # Declares test modules
│       ├── module_tests.rs
│       └── submodule_tests.rs
```

### Pattern

1. Create a `tests/` subdirectory within the module
2. Add `#[cfg(test)] mod tests;` at the bottom of `mod.rs`
3. Create `tests/mod.rs` to declare test submodules
4. Write tests in separate files (e.g., `tests/feature_tests.rs`)

### Example

```rust
// src/agents/tools/mod.rs (at the end)
#[cfg(test)]
mod tests;

// src/agents/tools/tests/mod.rs
mod git_blame_tests;
mod git_show_tests;
mod repo_map_tests;
mod static_analysis_tests;

// src/agents/tools/tests/repo_map_tests.rs
use crate::agents::tools::repo_map::{RepoMapArgs, RepoMapTool};

#[test]
fn repo_map_ranks_mentioned_files_and_extracts_symbols() {
    // ... real test from the codebase
}
```

### Why?

- Keeps source files focused on implementation
- Makes tests easier to find and navigate
- Reduces file size and cognitive load
- Follows the pattern established in `src/studio/tests/`

## Provider Configuration

Set via environment or config:

```bash
export OPENAI_API_KEY=sk-...
export ANTHROPIC_API_KEY=sk-ant-...
export GOOGLE_API_KEY=...
```

Or use CLI:

```bash
git-iris config --provider anthropic --api-key YOUR_KEY
git-iris config --provider anthropic --model claude-opus-5
```

### Provider Details

| Provider  | Default Model    | Fast Model                | Context |
| --------- | ---------------- | ------------------------- | ------- |
| openai    | gpt-6-astra      | gpt-5.6-luna              | 1.05M   |
| anthropic | claude-opus-5    | claude-haiku-4-5-20251001 | 1M      |
| google    | gemini-3.8-flash | gemini-3.5-flash-lite     | 1M      |

OpenAI defaults are workflow-aware: main agent generations use medium reasoning, subagents
use low reasoning, and status messages use none unless the provider config explicitly overrides
`reasoning`. Anthropic Opus 5 uses high effort for main tasks and low for subagents.
OpenRouter and Fireworks are supported through the same agent construction path. The optional
`subagent_model` selects delegated analysis independently of `fast_model` status messages.

## Key Design Decisions

1. **LLM-First**: No hardcoded heuristics—Iris makes decisions
2. **Tool-Based Context**: Gather only what's needed via tool calls
3. **Reducer-Centric Event Flow**: State changes stay predictable even though handlers and `StudioApp` still own some direct updates
4. **Structured Output**: JSON schemas ensure parseable responses
5. **Output Validation**: Recovery logic handles malformed JSON
6. **Unified Interface**: Studio provides one TUI for all operations
7. **Event-Driven**: Clear data flow from input to state to render
