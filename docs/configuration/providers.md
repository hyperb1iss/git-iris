# LLM Providers

Git-Iris supports OpenAI, Anthropic, Google, OpenRouter, and Fireworks.

## Provider Overview

| Provider      | Default Model      | Fast Model                  | Context Window | API Key Env         |
| ------------- | ------------------ | --------------------------- | -------------- | ------------------- |
| **OpenAI**    | `gpt-6-astra`      | `gpt-5.6-luna`              | 1.05M          | `OPENAI_API_KEY`    |
| **Anthropic** | `claude-opus-5`    | `claude-haiku-4-5-20251001` | 1M             | `ANTHROPIC_API_KEY` |
| **Google**    | `gemini-3.8-flash` | `gemini-3.5-flash-lite`     | 1M             | `GOOGLE_API_KEY`    |

OpenRouter defaults to `anthropic/claude-opus-5` and uses `OPENROUTER_API_KEY`. Fireworks defaults
to `accounts/fireworks/models/deepseek-v4-pro-0813` and uses `FIREWORKS_API_KEY`.

## Configuration Format

Each provider has its own section under `[providers]`:

```toml
[providers.PROVIDER_NAME]
api_key = "YOUR_API_KEY"
model = "model-name"           # Optional: primary model
fast_model = "fast-model-name" # Optional: for status updates
subagent_model = "worker-model-name" # Optional: defaults to the primary model
token_limit = 1050000          # Optional: context-window metadata
```

## OpenAI Configuration

```toml
[providers.openai]
api_key = "sk-..."
model = "gpt-6-astra"
fast_model = "gpt-5.6-luna"
```

### CLI Setup

```bash
git-iris config --provider openai --api-key YOUR_API_KEY
git-iris config --provider openai --model gpt-6-astra
```

### Environment Variable

```bash
export OPENAI_API_KEY="sk-..."
```

## Anthropic Configuration

```toml
[providers.anthropic]
api_key = "sk-ant-..."
model = "claude-opus-5"
fast_model = "claude-haiku-4-5-20251001"
```

### CLI Setup

```bash
git-iris config --provider anthropic --api-key YOUR_API_KEY
git-iris config --provider anthropic --model claude-opus-5
```

### Environment Variable

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

### Legacy Aliases

The provider names `claude` and `gemini` are still supported as aliases for `anthropic` and `google`.

## Google Configuration

```toml
[providers.google]
api_key = "your-google-api-key"
model = "gemini-3.8-flash"
fast_model = "gemini-3.5-flash-lite"
```

### CLI Setup

```bash
git-iris config --provider google --api-key YOUR_API_KEY
git-iris config --provider google --model gemini-3.8-flash
```

### Environment Variable

```bash
export GOOGLE_API_KEY="..."
```

## OpenRouter Configuration

OpenRouter routes requests to hosted models using its own model IDs and API key. Iris uses the
native OpenRouter integration to retain reasoning details across tool calls.

```bash
export OPENROUTER_API_KEY="sk-or-..."
git-iris config --provider openrouter --model anthropic/claude-opus-5
```

```toml
[providers.openrouter]
model = "anthropic/claude-opus-5"
fast_model = "anthropic/claude-haiku-4.5"
```

The Opus default uses high effort for main tasks and low effort for subagents. Use model IDs from
[OpenRouter's catalog](https://openrouter.ai/models) and choose models with tool-calling support.

## Fireworks Configuration

Fireworks uses its OpenAI-compatible Chat Completions endpoint. Model IDs include the account path.

```bash
export FIREWORKS_API_KEY="..."
git-iris config --provider fireworks --model accounts/fireworks/models/deepseek-v4-pro-0813
```

```toml
[providers.fireworks]
model = "accounts/fireworks/models/deepseek-v4-pro-0813"
fast_model = "accounts/fireworks/models/deepseek-v4-flash-0731"
```

For the default DeepSeek V4 models, Iris uses high effort for analysis and disables thinking for
status messages. Fireworks promotes low and medium effort to high for this model family. Custom
models retain their provider defaults unless you supply parameters.
See [Fireworks' model guide](https://docs.fireworks.ai/guides/recommended-models) for serverless
availability and supported features. An account-specific deployment can use its own model ID.

## Switching Providers

### Set Default Provider

```bash
git-iris config --provider anthropic
```

### Override Per-Command

```bash
git-iris gen --provider openai
git-iris review --provider google
```

## Additional Parameters

Provider-specific parameters can be set using `--param`:

```bash
git-iris config --provider openai --param reasoning='{"effort":"medium"}'
git-iris config --provider openai --param text='{"verbosity":"low"}'
```

Git-Iris parses valid JSON values here, so nested provider options work without extra config
files. The `--token-limit` setting records context-window metadata; it does not change output
budgets. Iris currently requests 16,384 output tokens for main tasks and 4,096 for subagents.

OpenAI defaults use the Responses API and choose reasoning by workflow:

- Main agent generations use `reasoning = {"effort":"medium"}`
- Subagents and `parallel_analyze` use `reasoning = {"effort":"low"}`
- Fast status messages use Luna with `reasoning = {"effort":"none"}`

Set `reasoning` yourself only when you want to override those defaults for every OpenAI request
using that provider config.

In TOML:

```toml
[providers.openai]
api_key = "sk-..."

  [providers.openai.additional_params]
  reasoning = '{"effort":"medium"}'
  text = '{"verbosity":"low"}'
```

## Token Limits

The model context window and output-token budget are separate limits. See
[Model Selection](./models) for role defaults and context guidance.

## API Key Priority

API keys are loaded in this order:

1. **Config file** (`~/.config/git-iris/config.toml`)
2. **Environment variable** (`OPENAI_API_KEY`, etc.)

Project configs (`.irisconfig`) **never** contain API keys for security.

## Verification

Check your provider configuration:

```bash
# View current configuration
cat ~/.config/git-iris/config.toml

# Test with a command
git-iris gen --print
```

If authentication fails, Git-Iris will tell you which environment variable to set.

## Security Best Practices

- **Never commit** `config.toml` with API keys
- Use environment variables in CI/CD
- Restrict file permissions:
  ```bash
  chmod 600 ~/.config/git-iris/config.toml
  ```
- Rotate API keys periodically
- Use separate keys for different projects if needed
