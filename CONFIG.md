# Git-Iris Configuration File

The Git-Iris configuration file is stored in `~/.git-iris` and uses the TOML format. This document describes the available configuration options and syntax.

## Configuration Structure

The configuration file is organized into several sections:

1. Global settings
2. Default provider
3. Provider-specific configurations

## Configuration Options

### Global Settings

- `use_gitmoji`: Boolean (optional)
  - Description: Whether to include gitmoji in commit messages.
  - Example: `use_gitmoji = true`

- `custom_instructions`: String (optional)
  - Description: Custom instructions to be included in the prompt for all LLMs.
  - Example: `custom_instructions = "Always mention the ticket number if applicable."`

### Default Provider

- `default_provider`: String (required)
  - Description: The default LLM provider to use.
  - Example: `default_provider = "openai"`

### Provider-Specific Configurations

Provider configurations are stored under the `[providers]` table. Each provider has its own subtable with the following fields:

- `api_key`: String (required)
  - Description: The API key for the provider.

- `model`: String (required)
  - Description: The specific model to use for this provider.

- `additional_params`: Table (optional)
  - Description: Additional parameters specific to the provider or model.

## Example Configuration File

```toml
use_gitmoji = true
custom_instructions = """
Always mention the ticket number if applicable.
Use imperative mood in the commit message.
"""
default_provider = "openai"

[providers.openai]
api_key = "sk-1234567890abcdef"
model = "gpt-3.5-turbo"
additional_params = { temperature = "0.7", max_tokens = "150" }

[providers.claude]
api_key = "sk-abcdef1234567890"
model = "claude-v1"
additional_params = { temperature = "0.8" }
```

## Changing Configuration

You can change the configuration using the `git-iris config` command:

```
git-iris config --provider openai --api-key YOUR_API_KEY
git-iris config --provider openai --model gpt-3.5-turbo
git-iris config --provider openai --param temperature=0.7 --param max_tokens=150
git-iris config --gitmoji true
git-iris config --custom-instructions "Your custom instructions here"
```

Alternatively, you can edit the `~/.git-iris` file directly using a text editor.

## Adding a New Provider

To add a new provider, simply add a new section under `[providers]` with the provider's name:

```toml
[providers.new_provider]
api_key = "your-api-key-here"
model = "model-name"
additional_params = { param1 = "value1", param2 = "value2" }
```

Then, you can set it as the default provider if desired:

```toml
default_provider = "new_provider"
```

Remember to update the application code to support the new provider's API if it's not already implemented.