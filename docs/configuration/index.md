# Configuration Overview

Git-Iris uses a layered configuration system that combines personal settings, project-specific settings, and runtime overrides.

## Configuration Hierarchy

1. **Personal Config** (`~/.config/git-iris/config.toml`) — Your global defaults
2. **Project Config** (`.irisconfig` in repo root) — Team-shared settings
3. **Environment Variables** — API keys and integration-specific overrides
4. **CLI Flags** — Command-specific overrides

Settings are merged in this order (later takes precedence), except **API keys are never loaded from project config** for security.

## Quick Start

```bash
# Set up your provider (OpenAI is the default)
git-iris config --provider openai --api-key YOUR_OPENAI_API_KEY
# or:
git-iris config --provider anthropic --api-key YOUR_ANTHROPIC_API_KEY
git-iris config --provider google --api-key YOUR_GOOGLE_API_KEY

# Optionally override models for the selected provider
git-iris config --provider openai --model gpt-6-astra
git-iris config --provider openai --fast-model gpt-5.6-luna

# Enable gitmoji
git-iris config --gitmoji
```

## Configuration Files

### Personal Config Location

**macOS/Linux:**

```
~/.config/git-iris/config.toml
```

If `$XDG_CONFIG_HOME` is set, Git-Iris uses `$XDG_CONFIG_HOME/git-iris/config.toml` instead. On macOS, an existing `~/Library/Application Support/git-iris/config.toml` from older releases is still honored, but new installs default to the XDG-style path.

**Windows:**

Git-Iris uses the same XDG-style resolver on Windows. When `$HOME` is set (typical for Git Bash, WSL, and most shells), the config file lives at:

```
%HOME%\.config\git-iris\config.toml
```

If `$XDG_CONFIG_HOME` is set, that takes precedence (`%XDG_CONFIG_HOME%\git-iris\config.toml`). The platform's `%APPDATA%\git-iris\config.toml` path is only used as a last-resort fallback when `$HOME` cannot be determined.

### Project Config Location

```
.irisconfig  (in repository root)
```

## Configuration Sections

| Section       | Description                                                  | Scope                 |
| ------------- | ------------------------------------------------------------ | --------------------- |
| **Global**    | `use_gitmoji`, `instructions`, `instruction_preset`, `theme` | All operations        |
| **Provider**  | `default_provider`                                           | Which LLM to use      |
| **Providers** | `api_key`, `model`, `fast_model`, `token_limit`              | Per-provider settings |

## Basic Configuration Structure

```toml
# Global settings
use_gitmoji = true
instruction_preset = "conventional"
theme = "silkcircuit-neon"

# Default provider
default_provider = "openai"

# Provider configurations
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

## Global Settings

| Setting                 | Type    | Default     | Description                                          |
| ----------------------- | ------- | ----------- | ---------------------------------------------------- |
| `use_gitmoji`           | Boolean | `true`      | Enable emoji prefixes in commit messages             |
| `instructions`          | String  | `""`        | Custom instructions for all LLM operations           |
| `instruction_preset`    | String  | `"default"` | Built-in instruction preset name                     |
| `theme`                 | String  | `""`        | Theme name; empty means default (`silkcircuit-neon`) |
| `critic_enabled`        | Boolean | `true`      | Run critic verification for long-form artifacts      |
| `subagent_timeout_secs` | Integer | `120`       | Timeout in seconds for parallel subagent tasks       |
| `subagent_max_turns`    | Integer | `20`        | Per-subagent turn budget for `parallel_analyze`      |
| `default_provider`      | String  | `"openai"`  | Default LLM provider                                 |

## Next Steps

- **[Providers](providers.md)** — Configure OpenAI, Anthropic, or Google
- **[Models](models.md)** — Choose the right model for your needs
- **[Project Config](project-config.md)** — Share settings with your team
- **[Environment Variables](environment.md)** — Runtime configuration
