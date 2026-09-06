# Adding Capabilities

Capabilities define what tasks Iris can perform. Each capability is a TOML file containing a prompt and output schema. This guide shows you how to add a new capability to Git-Iris.

## What is a Capability?

A capability combines three elements:

1. **Task Prompt** — Instructions for Iris on how to approach the task
2. **Output Type** — Structured format for the response (JSON schema)
3. **Tool Access** — Which tools Iris can use for this task

## Capability Structure

### Basic TOML Format

```toml
name = "my_capability"
description = "Brief description of what this does"
output_type = "MyOutputType"

task_prompt = """
Instructions for Iris go here.

## Tools Available
- `git_diff()` - Get code changes
- `file_read(path="...")` - Analyze specific files

## Output Requirements
Your requirements for the output format.

## JSON Output
Return a `MyOutputType` with: field1, field2, etc.
"""
```

### Real Example: Commit Message Generation

From `src/agents/capabilities/commit.toml`:

```toml
name = "commit"
description = "Generate commit messages from staged changes"
output_type = "GeneratedMessage"

task_prompt = """
Generate a commit message for the staged changes.

## Context Gathering
`project_docs(doc_type="context")` returns a compact snapshot of the README and agent instructions.
Start with `git_diff()` for change evidence, then call `project_docs` when repository conventions or product framing matter.

## Tools Available
- `project_docs(doc_type="context")` - Compact project conventions snapshot; use targeted doc types for full docs
- `git_diff()` - Get staged changes with relevance scores
- `git_log(count=5)` - Recent commits for style reference

## Workflow
1. Call `git_diff()` to see what changed
2. Call `project_docs(doc_type="context")` when repository conventions affect the wording
3. Generate the commit message following project conventions

## Output Requirements
- **Subject line**: Imperative mood, max 72 chars, no period
- **Body**: Explain WHY, not what. Wrap at 72 chars.
- **Plain text only**: No markdown, no code fences

## JSON Output
Return a `GeneratedMessage` with: `emoji` (string or null), `title` (subject), `message` (body)
"""
```

## Step-by-Step: Adding a New Capability

::: tip Teaching Example
This section walks through creating a hypothetical "Feature Summary" capability. **This capability does not exist in the current codebase** — it's an example to illustrate the pattern. Follow along to learn how capabilities work.
:::

### Step 1: Create the TOML File

Create `src/agents/capabilities/feature_summary.toml`:

```toml
name = "feature_summary"
description = "Generate a high-level summary of a feature branch"
output_type = "FeatureSummary"

task_prompt = """
You are Iris, an AI assistant analyzing a feature branch to create a high-level summary.

## Context Gathering
`project_docs(doc_type="context")` returns a compact project snapshot. Use it when repo terminology, conventions, or workflow rules affect the summary.

## Tools Available
- `project_docs(doc_type="context")` - Get a compact project context snapshot
- `git_diff(from="<default-branch>", to="HEAD")` - Get changes between branches
- `git_log(count=N)` - Get commit history
- `file_read(path="...")` - Analyze specific files in detail

## Workflow
1. Get the diff between the primary branch and the feature branch
2. Call `project_docs(doc_type="context")` when repository conventions or terminology affect the summary
3. Identify key files and patterns
4. Summarize the feature's purpose, implementation approach, and impact

## Output Requirements
- **Purpose**: 1-2 sentences on what this feature does
- **Approach**: Brief technical overview
- **Files Changed**: Count and categorization
- **Impact**: User-facing changes, API changes, internal refactors

## JSON Output
Return a `FeatureSummary` with:
- `purpose` (string)
- `approach` (string)
- `files_changed` (number)
- `impact` (string)
"""
```

### Step 2: Define the Output Type

Create or update `src/types/feature_summary.rs`:

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Feature summary response
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct FeatureSummary {
    /// What this feature does (1-2 sentences)
    pub purpose: String,
    /// Technical approach overview
    pub approach: String,
    /// Number of files changed
    pub files_changed: usize,
    /// User-facing and technical impact
    pub impact: String,
}

impl FeatureSummary {
    /// Format as markdown for display
    pub fn format(&self) -> String {
        format!(
            "# Feature Summary\n\n\
            ## Purpose\n{}\n\n\
            ## Approach\n{}\n\n\
            ## Impact\n{}\n\n\
            Files changed: {}\n",
            self.purpose,
            self.approach,
            self.impact,
            self.files_changed
        )
    }
}
```

Add to `src/types/mod.rs`:

```rust
pub mod feature_summary;
pub use feature_summary::FeatureSummary;
```

### Step 3: Register in StructuredResponse

Edit `src/agents/iris.rs`. The shipped variants today are `CommitMessage`, `PullRequest`, `Changelog`, `ReleaseNotes`, `Review`, `SemanticBlame`, and `PlainText` — note that the review arm is named `Review`, not `MarkdownReview`, and wraps `crate::types::Review` (a structured type, not a markdown wrapper):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StructuredResponse {
    CommitMessage(crate::types::GeneratedMessage),
    PullRequest(crate::types::MarkdownPullRequest),
    Changelog(crate::types::MarkdownChangelog),
    ReleaseNotes(crate::types::MarkdownReleaseNotes),
    /// Structured code review with parseable findings
    Review(crate::types::Review),
    /// Semantic blame explanation (plain text)
    SemanticBlame(String),
    PlainText(String),
    // Add your new type:
    FeatureSummary(crate::types::FeatureSummary),
}

impl fmt::Display for StructuredResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // ... existing arms ...
            StructuredResponse::FeatureSummary(summary) => {
                write!(f, "{}", summary.format())
            }
        }
    }
}
```

Also extend `serialize_artifact_for_critic` if the critic should be able to inspect your new variant — see [Critic Verification](#critic-verification-internal-output-types) below.

### Step 4: Load the Capability

Add the embedded TOML constant in `src/agents/iris.rs` alongside the existing ones:

```rust
const CAPABILITY_COMMIT: &str = include_str!("capabilities/commit.toml");
const CAPABILITY_PR: &str = include_str!("capabilities/pr.toml");
// ... existing capabilities ...
const CAPABILITY_FEATURE_SUMMARY: &str = include_str!("capabilities/feature_summary.toml");
```

Then register the new capability inside `load_capability_config()`. The loader uses a plain `match` against the capability name, not a HashMap insertion:

```rust
let content = match capability {
    "commit" => CAPABILITY_COMMIT,
    "pr" => CAPABILITY_PR,
    "review" => CAPABILITY_REVIEW,
    "changelog" => CAPABILITY_CHANGELOG,
    "release_notes" => CAPABILITY_RELEASE_NOTES,
    "chat" => CAPABILITY_CHAT,
    "semantic_blame" => CAPABILITY_SEMANTIC_BLAME,
    "feature_summary" => CAPABILITY_FEATURE_SUMMARY,  // Add your arm
    _ => {
        // Unknown capabilities fall back to a generic prompt + PlainText output type
        return Ok((
            format!(
                "You are helping with a {capability} task. Use the available Git tools to assist the user."
            ),
            "PlainText".to_string(),
        ));
    }
};
```

### Step 5: Add Execution Logic

Iris dispatches on the capability's declared `output_type` (the string in the TOML), not on the capability name. Find `execute_output_type(&self, output_type: &str, system_prompt: &str, user_prompt: &str)` in `src/agents/iris.rs` and add a match arm. It delegates the schema-driven call to the internal `execute_with_agent::<T>()` helper:

```rust
async fn execute_output_type(
    &self,
    output_type: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<StructuredResponse> {
    match output_type {
        "GeneratedMessage" => {
            let response = self
                .execute_with_agent::<crate::types::GeneratedMessage>(
                    system_prompt,
                    user_prompt,
                )
                .await?;
            Ok(StructuredResponse::CommitMessage(response))
        }
        // ... existing arms (MarkdownPullRequest, MarkdownChangelog, MarkdownReleaseNotes, Review, SemanticBlame) ...
        "FeatureSummary" => {
            let response = self
                .execute_with_agent::<crate::types::FeatureSummary>(
                    system_prompt,
                    user_prompt,
                )
                .await?;
            Ok(StructuredResponse::FeatureSummary(response))
        }
        _ => {
            let agent = self.build_agent()?;
            let full_prompt = format!("{system_prompt}\n\n{user_prompt}");
            let response = agent.prompt_multi_turn(&full_prompt, 50).await?;
            Ok(StructuredResponse::PlainText(response))
        }
    }
}
```

### Step 6: Test Your Capability

```bash
# Build
just build

# Test in CLI (you may need to add a CLI command for your capability)
just run -- feature-summary main..feature-branch

# Or test in Studio (if you add a mode for it)
just studio
```

## Best Practices

### Prompt Engineering

**State the evidence needed without forcing a tool sequence:**

```toml
## Evidence
Use the selected comparison scope for all tools and delegated tasks.
Inspect the patches that support material claims. Use a compact summary to orient broad changes,
then filtered diffs and targeted reads. Use project_docs(doc_type="context") when conventions matter.
Account for the requested scope and name evidence gaps. Return the artifact in its supplied schema.
```

**Provide clear output requirements:**

```toml
## Output Requirements
- **Title**: Max 100 chars, action-oriented
- **Summary**: 2-3 paragraphs, focus on impact
- **Be precise**: State verified facts clearly and call out inferences when evidence is incomplete
```

**Give examples:**

```toml
Example output:
{
  "title": "Add user authentication with JWT",
  "summary": "Implements JWT-based authentication...",
  "impact": "Breaking: All API endpoints now require auth headers"
}
```

### Context Strategy

Guide investigation without excluding parts of the requested scope:

```toml
## Evidence Coverage
Use summaries to orient broad changes, then inspect supporting patches and affected contracts.
Relevance scores guide investigation order, not which files count toward coverage.
Delegate independent questions when their answers improve coverage.
```

### Tool Selection

Only list tools relevant to the task:

```toml
## Tools Available
- `git_diff()` - Get changes (use detail="summary" first)
- `git_log(count=5)` - Recent commits for context
- `file_read(path="...")` - Deep analysis of specific files
# Don't list every possible tool—keep it focused
```

### Certainty Standards

Calibrate claims to evidence:

```toml
## Writing Standards
- Be precise about confidence. If evidence is incomplete, gather more context and call out what is verified versus inferred.
- If unsure, use tools to gather more context
```

## Output Type Patterns

### Simple JSON Schema

For structured data:

```rust
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct SimpleOutput {
    pub field1: String,
    pub field2: Vec<String>,
    pub field3: Option<usize>,
}
```

### Markdown Wrapper

For LLM-driven formatting:

```rust
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct MarkdownOutput {
    /// Markdown content (LLM controls structure)
    pub content: String,
}

impl MarkdownOutput {
    pub fn raw_content(&self) -> &str {
        &self.content
    }
}
```

Use markdown wrappers when you want the LLM to control the exact structure while still having parseable output.

## Common Patterns

### Multi-Stage Analysis

```toml
## Workflow
1. Initial scan: `git_diff(detail="summary")` for overview
2. Identify key areas from relevance scores
3. Inspect the patches and callers needed to support material claims
4. Synthesize into structured output
```

### Parallel Processing

For independent questions that benefit from concurrent investigation:

```toml
## Independent Investigations
Use `parallel_analyze` to distribute work:
parallel_analyze({
  "tasks": [
    "Analyze API changes in src/api/",
    "Review database schema changes",
    "Check frontend component updates"
  ]
})
Supply exact refs and task constraints to every worker, then reconcile findings against evidence.
```

### Style Adaptation

Allow preset-based customization:

```toml
## Style Adaptation
If STYLE INSTRUCTIONS are provided, prioritize that style in your output.
The structural requirements still apply, but adapt tone and word choice.
```

## Critic Verification (Internal Output Types)

Not every `output_type` corresponds to a user-facing `StructuredResponse` variant. The shipped `verify` capability (`src/agents/capabilities/verify.toml`) returns a `Critique` value that is consumed entirely inside `iris.rs`:

- `should_run_critic(capability, output_type)` decides whether the critic should run for `(review, Review)`, `(pr, MarkdownPullRequest)`, `(changelog, MarkdownChangelog)`, or `(release_notes, MarkdownReleaseNotes)` by default. `(commit, GeneratedMessage)` runs only when `gen --critic` explicitly opts in.
- `verify_response_if_enabled()` loads the `verify` capability, runs `execute_with_agent::<Critique>()`, and either accepts the response or re-runs `execute_output_type()` with a revision prompt.
- `Critique` is a private internal type — it is never serialized into `StructuredResponse`.

Use this pattern when you want a capability that audits or post-processes another capability's artifact without surfacing its raw output to the user. Add the capability TOML, wire it into `load_capability_config()`, but call it explicitly from another method rather than threading it through `execute_output_type()`.

## Chat and Content Updates

The `chat` capability (`src/agents/capabilities/chat.toml`) is the conversational entry point users open with `/` in Studio. It does not return a `StructuredResponse` variant — instead, Iris mutates the active mode's artifact through dedicated tools defined in `src/agents/tools/content_update.rs`:

- `update_commit(emoji, title, message)` — rewrites the commit message
- `update_pr(content)` — rewrites the PR description
- `update_review(content)` — rewrites the review

Each tool ships a `ContentUpdate` value over an `mpsc::Sender<ContentUpdate>` that the Studio app drains on its event loop, applying the change to the relevant `*State` struct. When you want chat to manipulate a new artifact type, add a corresponding `Update<X>Tool` next to the existing trio, extend the `ContentUpdate` enum with the new variant, and handle the new variant in the Studio receiver. The chat capability's TOML lists these tools so the LLM knows when to invoke them.

## Integration with Studio

If you add a Studio mode for your capability:

1. Add mode variant to `Mode` enum in `src/studio/state/mod.rs`
2. Create state struct in `src/studio/state/modes.rs`
3. Implement handler and renderer (see [Adding Studio Modes](./modes.md))

## Troubleshooting

### Iris doesn't use the right tools

Make the tool list more explicit and add workflow steps that require specific tools.

### Output parsing fails

Ensure your JSON schema matches exactly. Test with `--debug` to see raw responses.

### Responses are too vague

Add certainty standards and specific output requirements. Show examples.

### Context is incomplete

Guide Iris on when to gather more context. Add size-based strategies.

## Examples in Codebase

Study these real capabilities:

- **`commit.toml`** — Simple structured output, style adaptation
- **`review.toml`** — Markdown wrapper, parallel analysis, size strategies
- **`pr.toml`** — Multi-stage workflow, context gathering
- **`changelog.toml`** — Version comparison, structured formatting

## Next Steps

- **Add tools** to give Iris new context sources → [Adding Tools](./tools.md)
- **Create a mode** to surface your capability in Studio → [Adding Studio Modes](./modes.md)
- **Contribute** your capability back → [Contributing](./contributing.md)
