# Project Configuration

Share Git-Iris settings across your team using `.irisconfig` files.

## Purpose

Project configs let you:

- **Standardize** commit style across the team
- **Enforce** consistent instruction presets
- **Configure** team-wide model preferences
- **Share** custom PR description instructions for the project

## Security: API Keys Never Stored

`.irisconfig` files **never** contain API keys. Each developer uses their own credentials from:

- Personal config (`~/.config/git-iris/config.toml`)
- Environment variables

This prevents credential leakage when committing `.irisconfig` to version control.

## Creating a Project Config

### Via CLI

```bash
# From repository root
git-iris project-config --provider anthropic
git-iris project-config --model claude-opus-5
git-iris project-config --preset conventional
```

This creates `.irisconfig` in your repo root.

### Manual Creation

Create `.irisconfig`:

```toml
# Team-shared Git-Iris configuration

# Gitmoji enforcement
use_gitmoji = true

# Default provider (API keys come from personal config)
default_provider = "anthropic"

# Instruction preset
instruction_preset = "conventional"

# Custom PR description instructions for this project
instructions = """
Always mention the ticket number in the format [PROJ-123].
Focus on business impact over implementation details.
"""

# Provider configurations (no API keys)
[providers.anthropic]
model = "claude-opus-5"
subagent_model = "claude-opus-5"
fast_model = "claude-haiku-4-5-20251001"
token_limit = 150000
```

## Configuration Layering

Settings are merged in this order:

1. **Personal config** (`~/.config/git-iris/config.toml`)
2. **Project config** (`.irisconfig`)
3. **CLI flags**

Later settings override earlier ones, **except API keys** which only come from personal config or environment.

### Example Layering

**Personal config:**

```toml
default_provider = "openai"
use_gitmoji = false

[providers.openai]
api_key = "sk-..."
model = "gpt-6-astra"
```

**Project config:**

```toml
default_provider = "anthropic"
use_gitmoji = true
instruction_preset = "conventional"

[providers.anthropic]
model = "claude-opus-5"
```

**Effective config:**

```toml
# From project config
default_provider = "anthropic"
use_gitmoji = true
instruction_preset = "conventional"

# From personal config (API key never in project config)
[providers.openai]
api_key = "sk-..."

# From project config
[providers.anthropic]
model = "claude-opus-5"
# API key loaded from personal config or ANTHROPIC_API_KEY env var
```

## Supported Project Settings

| Setting                 | Type    | Description                                     |
| ----------------------- | ------- | ----------------------------------------------- |
| `use_gitmoji`           | Boolean | Enable/disable gitmoji                          |
| `default_provider`      | String  | Team's preferred provider                       |
| `instruction_preset`    | String  | Shared instruction preset                       |
| `instructions`          | String  | Custom project instructions across capabilities |
| `theme`                 | String  | Team's preferred theme                          |
| `critic_enabled`        | Boolean | Run critic verification for long-form artifacts |
| `subagent_timeout_secs` | Integer | Timeout in seconds for parallel subagent tasks  |
| `subagent_max_turns`    | Integer | Per-subagent turn budget for `parallel_analyze` |

### Provider Settings (per provider)

| Setting             | Type    | Description              |
| ------------------- | ------- | ------------------------ |
| `model`             | String  | Primary model name       |
| `fast_model`        | String  | Fast model name          |
| `token_limit`       | Integer | Token limit override     |
| `additional_params` | Table   | Provider-specific params |

**API keys are excluded** from project configs automatically.

## Common Use Cases

### Conventional Commits Enforcement

```toml
use_gitmoji = false
instruction_preset = "conventional"
```

### PR Ticket Number Requirement

```toml
instructions = """
Always include the ticket number in the format [PROJ-123].
If no ticket exists, call that out in the reviewer notes.
"""
```

### Security-Focused Review Preset

```toml
instruction_preset = "security"
```

### Security-Focused PR Descriptions

```toml
instructions = """
Pay special attention to:
- Authentication and authorization
- Input validation
- SQL injection risks
- XSS vulnerabilities
"""
```

### Monorepo Configuration

```toml
instructions = """
This is a monorepo with multiple packages.
Always specify which package is affected:
- @app/frontend
- @app/backend
- @app/shared
"""
```

## Managing Project Config

### View Current Config

```bash
git-iris project-config --print
```

### Update Settings

```bash
# Change provider
git-iris project-config --provider google

# Change model
git-iris project-config --model gemini-3.8-flash

# Update token limit
git-iris project-config --token-limit 100000
```

### Edit Manually

```bash
# Edit .irisconfig directly
vim .irisconfig
```

### Remove Project Config

```bash
# Delete the file
rm .irisconfig

# Or git remove
git rm .irisconfig
```

## Version Control

### Should You Commit `.irisconfig`?

**Yes, if:**

- Your team wants consistent commit style
- You have project-specific PR description instructions
- You want to standardize on a provider/model

**Consider `.gitignore` if:**

- Each developer has different preferences
- The config is purely personal

### Recommended `.irisconfig`

```toml
# Commit to version control
use_gitmoji = true
instruction_preset = "conventional"
default_provider = "anthropic"

instructions = """
Project-specific PR description guidelines here.
"""

[providers.anthropic]
model = "claude-opus-5"
fast_model = "claude-haiku-4-5-20251001"

# API keys NOT included - loaded from personal config
```

## Team Onboarding

### Setup Instructions for New Team Members

1. **Install Git-Iris:**

   ```bash
   brew install hyperb1iss/tap/git-iris
   ```

2. **Configure personal API key:**

   ```bash
   git-iris config --provider anthropic --api-key YOUR_API_KEY
   ```

3. **Clone the repo:**

   ```bash
   git clone <repo-url>
   cd <repo>
   ```

4. **Git-Iris auto-detects `.irisconfig`:**
   ```bash
   git-iris gen
   # Uses project config automatically
   ```

## Troubleshooting

### Project Config Not Loading

Check that `.irisconfig` is in repository root:

```bash
git rev-parse --show-toplevel  # Find repo root
ls -la .irisconfig  # Check file exists
```

### API Key Still Required

Project config doesn't include API keys. Set via:

```bash
# Personal config
git-iris config --provider anthropic --api-key YOUR_API_KEY

# Or environment variable
export ANTHROPIC_API_KEY="sk-ant-..."
```

### Settings Not Applied

Check layering priority:

```bash
# View effective config
git-iris project-config --print

# CLI flags override everything
git-iris gen --provider openai  # Overrides project config
```

### Conflicting Settings

CLI flags take final precedence:

```
Personal Config -> Project Config -> CLI Flags
```

Example:

```bash
# .irisconfig sets provider = "anthropic"
# But this command uses openai:
git-iris gen --provider openai
```

## Examples

### Open Source Project

```toml
# .irisconfig for open source contributors

use_gitmoji = true
instruction_preset = "conventional"

instructions = """
Follow our CONTRIBUTING.md guidelines.
Reference issue numbers: Fixes #123
Call out reviewer focus areas.
"""
```

### Enterprise Application

```toml
# .irisconfig for corporate repo

default_provider = "anthropic"
instruction_preset = "detailed"

instructions = """
Include:
- JIRA ticket: [PROJ-123]
- Affected microservices
- Database migration status
- Feature flag names (if applicable)
"""

[providers.anthropic]
model = "claude-opus-5"
token_limit = 150000
```

### Microservice

```toml
# .irisconfig for payment-service

instructions = """
This is the payment-service microservice.
All PR descriptions should:
- Mention impact on payment flow
- Note PCI compliance implications
- Reference security review if needed
"""

instruction_preset = "security"
```
