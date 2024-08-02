# Git-Iris Configuration Guide

Git-Iris uses a TOML configuration file located at `~/.config/git-iris/config.toml`. This document outlines all available configuration options and their usage.

## Configuration Structure

The configuration file is organized into these main sections:

1. Global settings
2. Default provider
3. Provider-specific configurations

## Configuration Options

### Global Settings

- `use_gitmoji`: Boolean (optional)
  - Description: Enables Gitmoji in commit messages.
  - Default: `false`
  - Example: `use_gitmoji = true`

- `custom_instructions`: String (optional)
  - Description: Custom instructions included in all LLM prompts.
  - Default: `""`
  - Example: `custom_instructions = "Always mention the ticket number and focus on the impact of changes."`

### Default Provider

- `default_provider`: String (required)
  - Description: The default LLM provider.
  - Default: `"openai"`
  - Example: `default_provider = "claude"`

### Provider-Specific Configurations

Each provider has its own subtable under `[providers]` with these fields:

- `api_key`: String (required)
  - Description: The provider's API key.
  - Example: `api_key = "sk-1234567890abcdef"`

- `model`: String (optional)
  - Description: The specific model to use.
  - Default: Provider-dependent
  - Example: `model = "gpt-4o"`

- `additional_params`: Table (optional)
  - Description: Additional provider or model-specific parameters.
  - Example: `additional_params = { temperature = "0.7", max_tokens = "150" }`

- `custom_token_limit`: Integer (optional)
  - Description: Custom token limit for this provider.
  - Default: Provider-dependent
  - Example: `custom_token_limit = 8000`

## Supported Providers and Default Models

1. OpenAI
   - Default model: "gpt-o4"
2. Claude
   - Default model: "claude-3-sonnet-20240320"

## Example Configuration File

```toml
use_gitmoji = true
custom_instructions = """
Always mention the ticket number if applicable.
Focus on the impact of changes rather than implementation details.
"""
default_provider = "openai"

[providers.openai]
api_key = "sk-1234567890abcdef"
model = "gpt-4"
additional_params = { temperature = "0.7", max_tokens = "150" }
custom_token_limit = 8000

[providers.claude]
api_key = "sk-abcdef1234567890"
model = "claude-3-sonnet-20240320"
additional_params = { temperature = "0.8" }
custom_token_limit = 100000
```

## Changing Configuration

Use the `git-iris config` command to modify settings:

```bash
git-iris config --provider openai --api-key YOUR_API_KEY
git-iris config --provider openai --model gpt-4
git-iris config --provider openai --param temperature=0.7 --param max_tokens=150
git-iris config --gitmoji true
git-iris config --custom-instructions "Your custom instructions here"
git-iris config --token-limit 8000
```

You can also edit the `~/.config/git-iris/config.toml` file directly with a text editor.

## Adding a New Provider

To add a new provider, create a new section under `[providers]`:

```toml
[providers.new_provider]
api_key = "your-api-key-here"
model = "model-name"
additional_params = { param1 = "value1", param2 = "value2" }
custom_token_limit = 10000
```

Set it as the default provider if desired:

```toml
default_provider = "new_provider"
```

Note: The application code must support the new provider's API for it to function.

## Token Optimization

Git-Iris automatically optimizes token usage to maximize context while staying within provider limits. You can set a custom token limit for each provider using the `custom_token_limit` option.

## Security Notes

- Keep your API keys secret and never share your configuration file containing API keys.
- Git-Iris stores API keys in the configuration file. Ensure the file has appropriate permissions (readable only by you).
- Consider using environment variables for API keys in shared environments.

## Troubleshooting

If you encounter issues:

1. Verify your API keys are correct and have the necessary permissions.
2. Check that you're using supported models for each provider.
3. Ensure your custom instructions don't exceed token limits.
4. Review the Git-Iris logs for any error messages.

For further assistance, please refer to the [Git-Iris documentation](https://github.com/hyperb1iss/git-iris/wiki) or [open an issue](https://github.com/hyperb1iss/git-iris/issues) on the GitHub repository.