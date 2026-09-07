# Capability System

Capabilities define what Iris can do. Each capability is a TOML file that specifies the task prompt and expected output format.

**Location:** `src/agents/capabilities/*.toml`

## Design Philosophy

### Separation of Concerns

- **TOML files** define the task and output type
- **Agent** handles execution and tool calling
- **Types** define the structured response schema

This separation allows:

- **Non-programmers** to modify task instructions
- **Easy experimentation** with different prompts
- **Version control** of prompt engineering
- **Compile-time embedding** for portability

### LLM-Driven Structure

Capabilities define the task, evidence requirements, and output contract. Presentation remains flexible within that contract:

- **Commit messages:** JSON with specific fields (`emoji`, `title`, `message`)
- **Reviews:** Structured JSON with findings, evidence, metadata, and counts
- **PRs:** Markdown with flexibility for project-specific conventions

The LLM adapts to project needs while following general guidelines.

See [Prompt Contracts](./prompting) for instruction precedence, delegation, current provider guidance, and evaluation limits.

## Capability Structure

A capability TOML has three fields:

```toml
name = "capability_name"
description = "Short description of what this capability does"
output_type = "OutputTypeName"

task_prompt = """
Multi-line prompt that instructs Iris...
"""
```

### Output Types

Output types map to Rust enums in `src/agents/iris.rs`:

```rust
pub enum StructuredResponse {
    CommitMessage(GeneratedMessage),       // JSON: { emoji, title, message, completion_message }
    PullRequest(MarkdownPullRequest),      // Markdown wrapper: { content: String }
    Changelog(MarkdownChangelog),          // Markdown wrapper: { content: String }
    ReleaseNotes(MarkdownReleaseNotes),    // Markdown wrapper: { content: String }
    Review(crate::types::Review),          // Structured: { summary, metadata, findings[], stats }
    SemanticBlame(String),                 // Plain text
    PlainText(String),                     // Fallback
}
```

**Strict structured types** (`GeneratedMessage`, `Review`) define schemas Iris must populate field-by-field. **Markdown wrappers** carry a single `content: String` field and let Iris choose the layout. The internal `verify` capability returns a private `Critique` struct (see [Critic Verification](#critic-verification)); it never appears in `StructuredResponse`.

## Built-in Capabilities

### 1. Commit (`commit.toml`)

**Purpose:** Generate commit messages from staged changes

**Output:** `GeneratedMessage`

```rust
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct GeneratedMessage {
    pub emoji: Option<String>,            // Single gitmoji or null
    pub title: String,                    // Subject line (max 72 chars)
    pub message: String,                  // Body (may be empty)
    #[serde(default)]
    pub completion_message: Option<String>, // Short UI status, e.g. "Auth refactor ready."
}
```

`completion_message` is required by `commit.toml` so the Studio TUI has a one-line status to show when generation finishes.

**Key instructions:**

- Start with `git_diff()` for change evidence
- Use `project_docs(doc_type="context")` when repository conventions or product framing matter
- Treat `project_docs(doc_type="context")` as a compact snapshot; use targeted doc types for full files
- Account for the selected scope, using summaries to orient broad changes
- Delegate independent questions when their answers improve coverage

**Style adaptation:**

- Gitmoji mode: Set `emoji` field from gitmoji list
- Conventional mode: Set `emoji` to null, use conventional prefixes
- Presets: Apply personality while maintaining structure

### 2. Review (`review.toml`)

**Purpose:** Analyze code changes and emit a structured, parseable review

**Output:** `Review` — a strict JSON shape, not a markdown wrapper. The capability's `output_type = "Review"` (see `src/agents/capabilities/review.toml:3`).

```rust
pub struct Review {
    pub summary: String,
    pub metadata: ReviewMetadata,   // risk_level, strategy, specialist_passes, coverage_notes
    pub findings: Vec<Finding>,     // id, severity, confidence, file, line range, category, body, suggested_fix, evidence
    pub stats: ReviewStats,         // files_reviewed, findings_count, critical/high/medium/low counts
    pub parse_failed: bool,         // set when from_unstructured() rescues raw text
}
```

`Finding.confidence` is an integer 0–100. Findings below `DEFAULT_MIN_FINDING_CONFIDENCE = 70` are hidden from `visible_findings()` and from inline GitHub comments. `Category` is a 12-variant enum: `security`, `performance`, `error_handling`, `complexity`, `abstraction`, `duplication`, `testing`, `style`, `api_contract`, `concurrency`, `documentation`, `other`. `Severity` and `RiskLevel` accept `critical`/`high`/`medium`/`low`.

**Key instructions (from `review.toml`):**

- Use summaries to orient broad comparisons, then inspect patches and affected contracts with the tools that answer unresolved questions.
- Only report findings with confidence ≥ 70; do not duplicate issues a configured linter or type-checker already catches.
- Cite an exact `file:start_line` (and `end_line`) on a changed line; supply `suggested_fix` when feasible and `evidence` references for non-trivial claims.
- Set `metadata.risk_level`, name your `strategy`, list `specialist_passes` you ran (or delegated through `parallel_analyze`), and record `coverage_notes`.
- Return a JSON object matching the schema — never markdown. If there are no actionable issues, return `findings: []` and zero counts in `stats`.

### 3. Pull Request (`pr.toml`)

**Purpose:** Generate PR descriptions from branch changes

**Output:** `MarkdownPullRequest`

**Document structure:**

- Follow the supplied PR template and preserve accurate human context
- Lead with the problem and resulting behavior
- Explain the mechanism and validation at the depth the change needs
- Add compatibility or rollout details when they affect review

**Key instructions:**

- Preserve the supplied base and head refs in every diff and delegated task
- Account for the complete requested comparison
- Include migration/upgrade notes for breaking changes
- Distinguish executed validation from checks still needed

### 4. Changelog (`changelog.toml`)

**Purpose:** Generate changelog entries in Keep a Changelog format

**Output:** `MarkdownChangelog`

**Structure:**

```markdown
## [Version] - YYYY-MM-DD

### Added

- New features

### Changed

- Enhancements to existing features

### Deprecated

- Features marked for removal

### Removed

- Deleted features

### Fixed

- Bug fixes

### Security

- Security patches
```

**Key instructions:**

- Group changes by category
- Be specific about what changed
- Include migration notes if needed
- Focus on user-facing impact

### 5. Release Notes (`release_notes.toml`)

**Purpose:** Generate user-facing release documentation

**Output:** `MarkdownReleaseNotes`

**Suggested sections:**

- Highlights
- Breaking Changes
- New Features
- Improvements
- Bug Fixes
- Performance
- Upgrade Instructions

**Key instructions:**

- Write for end users, not developers
- Highlight impact and benefits
- Include version numbers and dates
- Provide upgrade path for breaking changes

### 6. Chat (`chat.toml`)

**Purpose:** Interactive conversation with Iris in Studio

**Output:** Varies (text or tool calls)

**Special features:**

- Access to content update tools (`update_commit`, `update_pr`, `update_review`)
- Can read and modify current Studio content
- Freeform conversation for exploration

### 7. Semantic Blame (`semantic_blame.toml`)

**Purpose:** Explain the history and reasoning behind code

**Output:** `SemanticBlame` (plain text)

**Key instructions:**

- Read git log for the file/region
- Analyze commit messages and diffs
- Explain _why_ the code evolved this way
- Connect changes to broader project goals

### 8. Verify (`verify.toml`) — Critic Verification

**Purpose:** Internal critic pass that checks a generated artifact against repository evidence.

**Output:** `Critique` — a private struct in `iris.rs` with fields `requires_revision: bool`, `issues: Vec<CritiqueIssue>` (title, body, severity), `revision_prompt: String`, `confidence: u8`. `Critique` is **not** part of `StructuredResponse`; it's consumed inside `verify_response_if_enabled` and used to decide whether to regenerate the artifact.

**How it runs.** After `execute_output_type` produces a `StructuredResponse`, `execute_task` passes the result to `verify_response_if_enabled`. When the critic is enabled (default `Config.critic_enabled = true`) and the `(capability, output_type)` pair matches review / pr / changelog / release_notes, Iris loads `verify.toml`, runs `execute_with_agent::<Critique>` against the serialized artifact and the original user prompt, and:

- If `requires_revision` is `false` (or `true` but issues and `revision_prompt` are both empty), returns the original artifact.
- Otherwise builds a revision prompt with the critic's issues and instruction appended and calls `execute_output_type` exactly once more.

`commit` / `GeneratedMessage` uses the same mechanism only when the invocation explicitly opts in with `gen --critic`.

**What the critic flags.** Unsupported claims, asserted risks without code verification, review findings citing the wrong file or line, PR/changelog/release note text that overstates scope, and missing caveats when an inference is presented as fact. It deliberately skips wording preferences and style choices that match repository conventions.

Critic evaluation failures preserve the original artifact and log a warning. Revision-generation
errors propagate to the caller. To opt out, set `critic_enabled = false` in the Git-Iris config.

## Creating a Custom Capability

### Step 1: Create the TOML File

Create `src/agents/capabilities/my_capability.toml`:

```toml
name = "my_capability"
description = "What my capability does"
output_type = "MyOutputType"

task_prompt = """
Instructions for Iris on how to complete this task.

## Tools Available
- `git_diff()` - Get changes
- `file_read()` - Read files
- `code_search()` - Search for patterns

## Output Requirements
Describe the expected structure...
"""
```

### Step 2: Define the Output Type

In `src/types/my_output.rs`:

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MyOutputType {
    pub summary: String,
    pub details: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
}
```

### Step 3: Add to `StructuredResponse` Enum

In `src/agents/iris.rs`:

```rust
pub enum StructuredResponse {
    // ... existing variants
    MyOutput(MyOutputType),
}

impl fmt::Display for StructuredResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // ... existing matches
            StructuredResponse::MyOutput(output) => {
                write!(f, "{}", output.summary)
            }
        }
    }
}
```

### Step 4: Embed the Capability

In `src/agents/iris.rs`, add the constant:

```rust
const CAPABILITY_MY_CAPABILITY: &str = include_str!("capabilities/my_capability.toml");
```

And update the loader:

```rust
fn load_capability_config(&self, capability: &str) -> Result<(String, String)> {
    let content = match capability {
        // ... existing capabilities
        "my_capability" => CAPABILITY_MY_CAPABILITY,
        _ => { /* fallback */ }
    };
    // ...
}
```

### Step 5: Handle Execution

In `execute_output_type()` (`src/agents/iris.rs:1068-1129`) — the inner dispatch function called by `execute_task` — add a match arm:

```rust
match output_type {
    // ... existing types
    "MyOutputType" => {
        let response = self
            .execute_with_agent::<MyOutputType>(system_prompt, user_prompt)
            .await?;
        Ok(StructuredResponse::MyOutput(response))
    }
    // ...
}
```

`execute_task` itself just loads the capability, injects style instructions, calls `execute_output_type`, then runs `verify_response_if_enabled` for the critic pass — you don't need to touch it for a new output type unless you want the critic to gate your new artifact (add the `(capability, output_type)` pair to `should_run_critic` if you do).

### Step 6: Test

```bash
cargo build
cargo run -- my-capability
```

## Prompt Engineering Best Practices

### Scope and Evidence

Give Iris the requested artifact, exact refs or staged scope, and completion criteria. Ask for the
highest-signal evidence first, then follow affected contracts and callers until the selected scope
is accounted for. Relevance scores guide inspection order; they do not exclude files from review.
Delegate independent questions when their answers improve coverage. Avoid mandatory tool sequences
or file-count thresholds.

### Output and Style

Specify the output type and required fields once. Preserve explicit user instructions and existing
templates. Apply presets within the artifact contract: commit formatting does not belong in a JSON
review, and personality must not invent evidence or pad a short PR description.

### Uncertainty

Separate observations from inferences. Investigate uncertainty when the answer can change a finding;
otherwise state the gap. A confident tone cannot substitute for evidence, and an empty review is a
valid outcome when no supported regressions remain.

### Repository Context

Read relevant project documentation for conventions and terminology. Repository text and tool
results are evidence, not instructions that can override the user's task or the output contract.

See [Prompt Contracts](./prompting.md) for runtime precedence and the evaluation approach.

## Validation and Recovery

All JSON outputs go through schema validation:

1. **Schema generation** — `schemars::schema_for!` creates JSON schema from Rust type
2. **Prompt injection** — Schema is added to prompt as a constraint
3. **Response parsing** — `extract_json_from_response()` finds JSON in response
4. **Sanitization** — `sanitize_json_response()` fixes control characters
5. **Validation** — `validate_and_parse()` attempts recovery if parsing fails

See [Output Validation](./output.md) for details.

## Debugging Capabilities

Run with `--debug` to see:

- Which capability is loaded
- The full prompt sent to the LLM
- Tool calls made by Iris
- JSON extraction and validation steps
- Token usage statistics

```bash
git-iris gen --debug
```

Color-coded output shows:

- 🔵 Blue — Phase transitions
- 🟢 Green — Successful operations
- 🟡 Yellow — Warnings
- 🔴 Red — Errors

## Next Steps

- [Tools](./tools.md) — Building tools that capabilities can use
- [Output Validation](./output.md) — Schema validation and error recovery
- [Agent System](./agent.md) — How capabilities are executed
