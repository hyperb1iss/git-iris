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
  - Default: `false`
  - Example: `use_gitmoji = true`

- `custom_instructions`: String (optional)
  - Description: Custom instructions to be included in the prompt for all LLMs.
  - Default: `""`
  - Example: `custom_instructions = "Always mention the ticket number if applicable."`

### Default Provider

- `default_provider`: String (required)
  - Description: The default LLM provider to use.
  - Default: `"openai"`
  - Example: `default_provider = "openai"`

### Provider-Specific Configurations

Provider configurations are stored under the `[providers]` table. Each provider has its own subtable with the following fields:

- `api_key`: String (required)
  - Description: The API key for the provider.
  - Example: `api_key = "sk-1234567890abcdef"`

- `model`: String (optional)
  - Description: The specific model to use for this provider.
  - If not specified, a default model will be used as determined by the provider.
  - Example: `model = "gpt-4o"`

- `additional_params`: Table (optional)
  - Description: Additional parameters specific to the provider or model.
  - Example: `additional_params = { temperature = "0.7", max_tokens = "150" }`

## Supported Providers and Default Models

Git-Iris currently supports the following providers:

1. OpenAI
   - Default model: "gpt-4o"
2. Claude
   - Default model: "claude-3-sonnet"

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
# model is optional and will use the provider's default if not specified
additional_params = { temperature = "0.7", max_tokens = "150" }

[providers.claude]
api_key = "sk-abcdef1234567890"
# model is optional and will use the provider's default if not specified
additional_params = { temperature = "0.8" }
```

## Changing Configuration

You can change the configuration using the `git-iris config` command:

```
git-iris config --provider openai --api-key YOUR_API_KEY
git-iris config --provider openai --model gpt-4o
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

## Notes

- If a configuration option is not specified in the file, Git-Iris will use the default value.
- The `model` field for each provider is optional. If not specified, Git-Iris will use the default model as determined by the provider.
- Always keep your API keys secret and never share your configuration file containing API keys.
- When adding custom instructions, be mindful of the token limits of the LLM models you're using.

For any issues or further questions about configuration, please refer to the Git-Iris documentation or open an issue on the project's GitHub repository.