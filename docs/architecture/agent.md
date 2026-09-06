# Iris Agent System

The Iris Agent is the core intelligence of Git-Iris, built on [Rig 0.42](https://docs.rs/rig/0.42.0) (the `rig` facade crate) for agentic workflows.

**Source:** `src/agents/iris.rs`

## Design Philosophy

### One Agent to Rule Them All

Git-Iris uses a **unified agent architecture** with capability switching:

```rust
pub struct IrisAgent {
    provider: String,              // Provider registry name
    model: String,                 // Primary model for complex tasks
    fast_model: Option<String>,    // Lightweight status model
    current_capability: Option<String>, // Active capability
    provider_config: HashMap<String, String>,
    preamble: Option<String>,
    config: Option<Config>,
    content_update_sender: Option<ContentUpdateSender>,
}
```

**Benefits:**

- **Code reuse** — Tools, validation, and execution logic shared across all capabilities
- **Consistency** — Same decision-making process for commits, reviews, PRs, etc.
- **Maintainability** — Fix a bug once, it's fixed everywhere
- **Testability** — Test one agent with different prompts

### Provider Construction

The provider module creates clients through one `agent_builder` entry point. Rig 0.42 erases the
completion model type inside its agent, so `DynAgent` wraps one shared agent type. OpenAI uses
Responses, OpenRouter uses its native adapter, and Fireworks uses an OpenAI-compatible Chat
Completions client. Anthropic and Gemini retain their native adapters.

API keys resolve from provider configuration, then the provider's environment variable. Service
entry points bind repository identity and execution trust for tool calls. Parallel workers carry
both values into spawned tasks; remote clones cannot run project build scripts through static analysis.

## Agent Lifecycle

### 1. Creation

```rust
// Direct creation
let agent = IrisAgent::new("anthropic", "claude-opus-5")?;

// With builder
let agent = IrisAgentBuilder::new()
    .with_provider("anthropic")
    .with_model("claude-opus-5")
    .with_preamble("Custom instructions...")
    .build()?;
```

### 2. Configuration

Agents can be configured with:

- **Subagent model** for delegated analysis: set `providers.<name>.subagent_model` in the config
- **Config** for gitmoji/presets: `agent.set_config(config)`
- **Content update sender** for Studio chat mode: `agent.set_content_update_sender(sender)`

### 3. Execution

```rust
let response = agent.execute_task("commit", "Generate a commit message").await?;

match response {
    StructuredResponse::CommitMessage(msg) => {
        println!("{} {}", msg.emoji.unwrap_or_default(), msg.title);
    }
    _ => {}
}
```

### 4. Streaming (for TUI)

```rust
agent.execute_task_streaming("review", prompt, |chunk, aggregated| {
    // Update TUI with each text chunk
    print!("{}", chunk);
}).await?;
```

## Tool Attachment

The shared `build_agent` path creates a focused subagent and main agent using the selected
provider. It applies `CompletionProfile::Subagent` and `CompletionProfile::MainAgent` through
`apply_completion_params`, then attaches their tools.

The main agent receives the core registry plus repository metadata, a persistent workspace,
`ParallelAnalyze`, and the focused subagent through Rig's `dynamic_tool` adapter. Studio chat adds
content-update tools when a sender is present. Subagents receive core tools without delegation.

Provider parameter mapping lives in `src/agents/provider.rs`. OpenAI gets Responses reasoning;
Opus gets adaptive thinking and effort; Gemini gets generation configuration. Anthropic's builder
enables automatic prompt caching. Explicit model choices and provider parameters remain configurable.

### Tool Registry Pattern

The `attach_core_tools!` macro ensures consistency. It wires **eleven** core tools — every Git source of evidence, file/code access, repository orientation, linter execution, and project documentation:

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
```

The companion `CORE_TOOLS: &[&str]` constant in `src/agents/tools/registry.rs` lists the same eleven names, and a unit test asserts the count stays at 11.

**Prevents drift** — Main agents and subagents always have the same core tools. Delegation tools (`Workspace`, `ParallelAnalyze`, the sub-agent itself) are attached only to the main agent so subagents can't recurse.

## Multi-Turn Execution

Iris operates in **multi-turn mode**, allowing up to 50 tool calls. The non-streaming path calls `prompt_extended` on `DynAgent`, which chains `max_turns(depth).extended_details()` on the shared agent:

```rust
let prompt_response: PromptResponse = agent.prompt_extended(&full_prompt, 50).await?;
// inside DynAgent::prompt_extended:
//   Self::OpenAI(a)    => a.prompt(msg).max_turns(depth).extended_details().await,
//   Self::Anthropic(a) => a.prompt(msg).max_turns(depth).extended_details().await,
//   Self::Gemini(a)    => a.prompt(msg).max_turns(depth).extended_details().await,
```

`.multi_turn()` is the streaming-only equivalent — the streaming path constructs a provider-specific `Agent<M>` first (see `build_*_agent_for_streaming`) and then calls `.stream_prompt(...).multi_turn(50).await`.

### Execution Flow

```mermaid
flowchart TB
    prompt[Receive Capability Prompt]
    decide[Decide Which Tools to Call]
    execute[Tools Return Structured Data]
    iterate[Call More Tools if Needed]
    response[Return Structured JSON]

    prompt --> decide
    decide --> execute
    execute --> iterate
    iterate --> response
```

| Step           | Action                            | Example                                               |
| -------------- | --------------------------------- | ----------------------------------------------------- |
| **1. Receive** | Load prompt from capability TOML  | "Generate a commit message for staged changes..."     |
| **2. Decide**  | Iris selects which tools to call  | `git_diff()`, `project_docs()`, `git_log(count=5)`    |
| **3. Execute** | Tools return structured data      | Diff with scores, compact doc context, recent commits |
| **4. Iterate** | Call more tools based on findings | `file_read()` to examine specific files               |
| **5. Return**  | Generate structured JSON response | `{ emoji: "✨", title: "...", message: "..." }`       |

**Why 50 turns?** Complex capabilities like PRs and release notes may need to:

- Analyze 20+ changed files individually
- Read commit history across a feature branch
- Search for patterns in configuration files
- Spawn parallel subagents for deep analysis

Iris knows when to stop, so we provide generous headroom.

## Capability Loading

Capabilities are embedded at compile time and loaded dynamically. There are **eight** capability TOML files: seven user-facing capabilities plus the internal `verify` capability used by the critic pass.

```rust
fn load_capability_config(&self, capability: &str) -> Result<(String, String)> {
    // The verify capability is handled separately so the critic pass can be cached
    if capability == "verify" {
        return Self::load_verify_capability_config(); // uses CAPABILITY_VERIFY
    }

    let content = match capability {
        "commit" => CAPABILITY_COMMIT,
        "pr" => CAPABILITY_PR,
        "review" => CAPABILITY_REVIEW,
        "changelog" => CAPABILITY_CHANGELOG,
        "release_notes" => CAPABILITY_RELEASE_NOTES,
        "chat" => CAPABILITY_CHAT,
        "semantic_blame" => CAPABILITY_SEMANTIC_BLAME,
        _ => return Ok(("Generic prompt".to_string(), "PlainText".to_string())),
    };

    let parsed: toml::Value = toml::from_str(content)?;
    let task_prompt = parsed.get("task_prompt")...;
    let output_type = parsed.get("output_type")...;

    Ok((task_prompt.to_string(), output_type))
}
```

The tuple `(task_prompt, output_type)` determines:

- **What Iris is asked to do** (the prompt)
- **What format to return** (JSON schema type)

### Critic Verification Pass

After `execute_output_type` returns a structured response, `execute_task` calls `verify_response_if_enabled`. When the critic is enabled (default `Config.critic_enabled = true`), Iris loads the `verify` capability — whose `output_type = "Critique"` — and runs it as an `execute_with_agent::<Critique>` call against the serialized artifact and the original task. The default set is reviews, PR descriptions, changelogs, and release notes; commit messages use the critic only when `gen --critic` opts in.

`Critique` has four fields: `requires_revision: bool`, `issues: Vec<CritiqueIssue>` (title, body, severity), `revision_prompt: String`, `confidence: u8`. If the critic returns `requires_revision = true` and provides either issues or a revision prompt, `execute_output_type` runs once more with the original system prompt and a user prompt augmented with the critic feedback. The pass runs only for output types where a critic check pays off:

```rust
matches!(
    (capability, output_type),
    ("commit", "GeneratedMessage")
        | ("review", "Review")
        | ("pr", "MarkdownPullRequest")
        | ("changelog", "MarkdownChangelog")
        | ("release_notes", "MarkdownReleaseNotes"),
)
```

Failures inside the critic (loading the capability, parsing the JSON, network errors) are logged as warnings and the original artifact is returned unchanged — the critic is a safety net, not a hard gate.

## Structured Output Generation

After tools are called, Iris must return valid JSON:

```rust
async fn execute_with_agent<T>(&self, system_prompt: &str, user_prompt: &str) -> Result<T>
where
    T: JsonSchema + DeserializeOwned + Serialize + Send + Sync + 'static,
{
    // Generate JSON schema for type T
    let schema = schema_for!(T);
    let schema_json = serde_json::to_string_pretty(&schema)?;

    // Instruct Iris to respond with JSON matching the schema
    let full_prompt = format!(
        "{system_prompt}\n\n{user_prompt}\n\n\
        === CRITICAL: RESPONSE FORMAT ===\n\
        REQUIRED JSON SCHEMA:\n{schema_json}\n\n\
        Your entire response should be ONLY the JSON object."
    );

    let prompt_response: PromptResponse = agent.prompt_extended(&full_prompt, 50).await?;
    let response = &prompt_response.output;

    // Extract and validate JSON
    let json_str = extract_json_from_response(response)?;
    let sanitized = sanitize_json_response(&json_str);
    let result: T = parse_with_recovery(sanitized.as_ref())?;

    Ok(result)
}
```

### JSON Extraction and Sanitization

**Extract:** Handles multiple formats

- Pure JSON: `{"emoji": "✨", ...}`
- Markdown code block: ` ```json\n{...}\n``` `
- With preamble: `Here's the commit message:\n{...}`

**Sanitize:** Fixes common LLM mistakes

- Literal newlines in strings → `\n`
- Unescaped control characters → Unicode escapes
- Tab characters → `\t`

**Validate:** Schema-aware recovery

- Missing fields → Add defaults
- Type mismatches → Coerce to expected type
- Null where not allowed → Replace with defaults

See [Output Validation](./output.md) for details.

## Style Injection

Iris adapts her output based on configuration:

```rust
fn inject_style_instructions(&self, system_prompt: &mut String, capability: &str) {
    let config = self.config?;
    let preset_name = config.get_effective_preset_name();
    let is_conventional = preset_name == "conventional";
    let gitmoji_enabled = config.use_gitmoji && !is_conventional;

    // Inject instruction preset
    if let Some(preset) = library.get_preset(preset_name) {
        system_prompt.push_str("\n\n=== STYLE INSTRUCTIONS ===\n");
        system_prompt.push_str(&preset.instructions);
    }

    // Handle gitmoji
    if gitmoji_enabled && capability == "commit" {
        system_prompt.push_str("\n\n=== GITMOJI INSTRUCTIONS ===\n");
        system_prompt.push_str("Set the 'emoji' field to a relevant gitmoji...");
        system_prompt.push_str(&get_gitmoji_prompt_guide());
    }
}
```

**Presets** like `cosmic` or `playful` inject personality into Iris's language while maintaining structural requirements (72-char limit, imperative mood, JSON format).

## Subagent Creation

Iris exposes a focused `analyze_subagent` tool and a `parallel_analyze` tool for independent tasks.
Both use `subagent_model` when configured, otherwise the primary model. The fast model is reserved
for status messages.

Subagents receive the core registry, a focused preamble, and 4,096 output tokens. They have no
further delegation tools. The primary agent receives 16,384 output tokens. Configured turn and
time budgets govern delegated tasks, and each worker retains the originating repository and trust.

Reasoning is selected by `CompletionProfile::Subagent`: low for Astra, Opus, and Gemini 3.8.
Fireworks DeepSeek V4 uses high because its API promotes low and medium to high.

## Provider-Specific Handling

### OpenAI Model Defaults

Git-Iris defaults to GPT-6 Astra for OpenAI analysis and GPT-5.6 Luna for status messages. Keep examples and docs aligned with the current defaults in `src/providers.rs` rather than older model aliases.

### Anthropic Prompt Caching

`anthropic_agent_builder` calls `.with_automatic_caching()` on every Anthropic completion model (`src/agents/provider.rs`). Rig places a top-level `cache_control` breakpoint on the last cacheable block, which the API advances as the conversation grows. On Iris's multi-turn tool loops, where every turn otherwise re-sends the whole transcript at full input price, cached turns are billed at a fraction of that cost. Caching is unconditional — prompts below the model's cacheable minimum are simply not cached by the API. Debug output surfaces both `cache_creation_input_tokens` and `cached_input_tokens` so you can confirm the cache is doing real work on your workload.

## Streaming Support

Streaming uses the same configured agent and tool registry as non-streaming generation:

```rust
let agent = self.build_agent()?;
let stream = agent.0.stream_prompt(&full_prompt).max_turns(50).await;
```

The consumer forwards text chunks and tool activity to Studio. The aggregated response is parsed
through `text_to_structured_response`, so the structured output contract stays consistent across
streaming and non-streaming tasks.

## Debug Instrumentation

All agent operations are instrumented for debugging:

```rust
use crate::agents::debug;

debug::debug_phase_change("AGENT EXECUTION: GeneratedMessage");
debug::debug_context_management("Agent built with tools", "Provider: anthropic...");
debug::debug_llm_request(&full_prompt, Some(16384));

let timer = debug::DebugTimer::start("Agent prompt execution");
let prompt_response = agent.prompt_extended(&full_prompt, 50).await?;
let response = &prompt_response.output;
timer.finish();

debug::debug_llm_response(&response, duration, Some(total_tokens));
debug::debug_json_parse_success("GeneratedMessage");
```

Enable with `--debug` flag for color-coded execution traces.

## Testing Patterns

### Unit Tests

Test capability loading:

```rust
#[test]
fn loads_commit_capability() {
    let agent = IrisAgent::new("openai", "gpt-6-astra").unwrap();
    let (prompt, output_type) = agent.load_capability_config("commit").unwrap();
    assert!(prompt.contains("Generate a commit message"));
    assert_eq!(output_type, "GeneratedMessage");
}
```

### Integration Tests

Test full execution with mocked tools:

```rust
#[tokio::test]
async fn generates_commit_message() {
    let agent = IrisAgent::new("openai", "gpt-6-astra").unwrap();
    let response = agent.execute_task("commit", "Generate message").await.unwrap();

    match response {
        StructuredResponse::CommitMessage(msg) => {
            assert!(!msg.title.is_empty());
        }
        _ => panic!("Wrong response type"),
    }
}
```

## Error Handling

Agent errors are propagated with context:

```rust
// Provider error
Err(anyhow::anyhow!("Failed to create agent builder for provider '{}': {}", provider, e))

// JSON parsing error
Err(anyhow::anyhow!("No valid JSON found in response"))

// Validation error
Err(anyhow::anyhow!("Failed to parse JSON even after recovery attempts: {}", e))
```

All errors flow through `anyhow::Result` for rich error context.

## Next Steps

- [Capabilities](./capabilities.md) — How to create custom task definitions
- [Tools](./tools.md) — Building and registering tools for Iris
- [Output Validation](./output.md) — Schema validation and error recovery
