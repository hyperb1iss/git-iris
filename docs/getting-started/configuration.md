# Configuration

Git-Iris uses a two-tier configuration system: global settings for your personal preferences, and project-specific settings that teams can share without exposing secrets.

## Global Configuration

Global settings live in `~/.config/git-iris/config.toml` and apply to all repositories.

### Set Up Your Provider

Pick your LLM provider and configure the API key:

**OpenAI:**

```bash
git-iris config --provider openai --api-key sk-...
```

**Anthropic:**

```bash
git-iris config --provider anthropic --api-key sk-ant-...
```

**Google:**

```bash
git-iris config --provider google --api-key AIza...
```

The API key is stored securely in your global config file.

### Environment Variables

Prefer environment variables? Git-Iris checks these:

```bash
export OPENAI_API_KEY=sk-...
export ANTHROPIC_API_KEY=sk-ant-...
export GOOGLE_API_KEY=AIza...
```

Environment variables take precedence over config file values.

### View Current Configuration

```bash
git-iris config
```

This displays your active provider, model, presets, and all configured providers.

## Provider Details

Git-Iris supports five providers. Direct-provider defaults are listed below; see
[Providers](../configuration/providers) for OpenRouter and Fireworks setup.

| Provider      | Default Model    | Fast Model                | Context Window | Best For                          |
| ------------- | ---------------- | ------------------------- | -------------- | --------------------------------- |
| **openai**    | gpt-6-astra      | gpt-5.6-luna              | 1.05M          | General purpose, fast             |
| **anthropic** | claude-opus-5    | claude-haiku-4-5-20251001 | 1M             | Deep analysis, code understanding |
| **google**    | gemini-3.8-flash | gemini-3.5-flash-lite     | 1M             | Massive context windows           |

**Fast models** are used for simple tasks like status updates and parsing. The primary model handles complex analysis.

### Override Models

Set a custom model for your provider:

```bash
git-iris config --provider anthropic --model claude-opus-5
```

Set a custom fast model:

```bash
git-iris config --provider openai --fast-model gpt-5.6-luna
```

### Token Limits

Override the default token limit:

```bash
git-iris config --provider openai --token-limit 4000
```

## Customization Options

### Gitmoji

Enable or disable emoji prefixes in commits:

```bash
git-iris config --gitmoji     # Enable
git-iris config --no-gitmoji  # Disable
```

### Critic Verification

Git-Iris runs a critic pass after long-form generated artifacts by default. The
critic checks whether the draft is supported by repository evidence and allows
one regeneration when it finds material overclaims.

The critic applies by default to reviews, PR descriptions, changelogs, and
release notes. Commit messages skip the critic unless you opt in with
`git-iris gen --critic`. It adds one extra model call, and a second extra call
only when the critic asks Iris to revise the draft.

To opt out globally, run `git-iris config --no-critic` or set
`critic_enabled = false` in your config file.

```bash
git-iris config --critic     # Enable
git-iris config --no-critic  # Disable

# Or disable it for one invocation
git-iris review --no-critic --print
```

### Instruction Presets

Set a default style preset:

```bash
git-iris config --preset conventional
git-iris config --preset detailed
git-iris config --preset cosmic
```

List available presets:

```bash
git-iris list-presets
```

Presets are categorized:

- **General** (both commits and reviews): `default`, `conventional`, `detailed`, `concise`, `cosmic`
- **Review-specific**: `security`, `performance`, `architecture`, `testing`, `maintainability`

### Custom Instructions

Add saved instructions for PR descriptions:

```bash
git-iris config --instructions "Always include a Validation section with exact commands."
```

These combine with presets when generating PR descriptions. For one-off instructions on any
command, pass `--instructions` directly to that command.

### Additional Parameters

Set provider-specific parameters:

```bash
git-iris config --provider openai \
  --param reasoning='{"effort":"medium"}' \
  --param text='{"verbosity":"low"}'
```

The `--token-limit` option records context-window metadata. It does not control generation output
budgets. See [Model Selection](../configuration/models) for role defaults.

## Project Configuration

Project settings live in `.irisconfig` in your repository root. Teams can commit this file to share settings **without exposing API keys**.

### Create Project Config

Set project-specific settings:

```bash
git-iris project-config --provider openai --preset conventional
```

Set a model for the project:

```bash
git-iris project-config --model gpt-6-astra
```

Set project PR instructions:

```bash
git-iris project-config --instructions "Call out migration blast radius explicitly."
```

### View Project Config

```bash
git-iris project-config --print
```

### Security Note

**API keys are NEVER stored in `.irisconfig` files.** Only non-sensitive settings like models, presets, and custom instructions are saved. This prevents accidentally committing secrets to version control.

API keys must be in your global config (`~/.config/git-iris/config.toml`) or environment variables.

## Configuration Precedence

Settings are layered with this priority:

1. **CLI flags** (highest priority)
2. **Environment variables**
3. **Project config** (`.irisconfig`)
4. **Global config** (`~/.config/git-iris/config.toml`)
5. **Defaults** (lowest priority)

Example:

```bash
# Project config sets anthropic, but CLI overrides for this run
git-iris gen --provider openai
```

## Themes

Git-Iris ships with five SilkCircuit variants (`silkcircuit-neon`, `silkcircuit-soft`, `silkcircuit-vibrant`, `silkcircuit-glow`, `silkcircuit-dawn`) plus 34 community themes from the [opaline](https://crates.io/crates/opaline) catalog.

There is no `git-iris config --theme` flag. Set your default theme by editing `~/.config/git-iris/config.toml`:

```toml
theme = "silkcircuit-glow"
```

List available themes (builtins + any installed in `~/.config/git-iris/themes/` or `~/.config/opaline/themes/`):

```bash
git-iris themes
```

`--theme` is a global flag, so it works on any subcommand for a single session:

```bash
git-iris studio --theme silkcircuit-dawn
git-iris gen --theme silkcircuit-vibrant
```

## Advanced Options

### Debug Mode

Enable color-coded agent execution output:

```bash
git-iris gen --debug
```

This shows Iris's tool calls, reasoning, and token usage in real-time with gorgeous formatting.

### Logging

Log debug messages to a file:

```bash
git-iris gen --log
```

Custom log file path:

```bash
git-iris gen --log --log-file /tmp/iris-debug.log
```

### Quiet Mode

Suppress spinners and progress messages:

```bash
git-iris gen --quiet
```

Useful for scripting and CI/CD where you only want final output.

## Example Workflows

### Team Setup

Commit shared project settings:

```bash
git-iris project-config --provider openai --preset conventional
git add .irisconfig
git commit -m "Add Git-Iris project configuration"
```

Team members clone the repo and only need to set their personal API key:

```bash
git-iris config --provider openai --api-key sk-...
```

### Multiple Providers

Switch providers easily:

```bash
git-iris config --provider openai --api-key sk-...
git-iris config --provider anthropic --api-key sk-ant-...

# Use OpenAI for this commit
git-iris gen --provider openai

# Use Anthropic for reviews
git-iris review --provider anthropic

# Use Google for large release notes
git-iris release-notes --provider google --from v1.0.0
```

### Per-Repository Presets

Different projects can use different styles:

```bash
cd backend-api
git-iris project-config --preset conventional

cd frontend-app
git-iris project-config --preset detailed
```

## Configuration File Format

The global config file (`~/.config/git-iris/config.toml`) looks like this:

```toml
default_provider = "openai"
use_gitmoji = true
critic_enabled = true
instruction_preset = "conventional"
instructions = "Always include validation receipts in PR descriptions"
theme = "silkcircuit-neon"

[providers.openai]
api_key = "sk-..."
model = "gpt-6-astra"
fast_model = "gpt-5.6-luna"

[providers.anthropic]
api_key = "sk-ant-..."
model = "claude-opus-5"
fast_model = "claude-haiku-4-5-20251001"

[providers.google]
api_key = "AIza..."
model = "gemini-3.8-flash"
fast_model = "gemini-3.5-flash-lite"
```

You can edit this manually if you prefer, but the `git-iris config` command is safer.

## What's Next?

With configuration complete, explore Git-Iris features:

- **[User Guide: Commits](/user-guide/commits)** — Master AI-generated commit messages
- **[User Guide: Reviews](/user-guide/reviews)** — Get detailed code reviews
- **[Iris Studio](/studio/)** — Learn all six Studio modes

Press `Shift+S` in Studio to adjust settings without leaving the interface.
