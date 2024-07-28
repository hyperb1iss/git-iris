# Git-Iris Configuration File

The Git-Iris configuration file is stored in `~/.git-iris` and uses the TOML format. This document describes the available configuration options.

## Configuration Options

### `api_key`

- Type: String
- Description: The API key for the chosen LLM provider (OpenAI or Anthropic).
- Example: `api_key = "sk-1234567890abcdef"`

### `llm_provider`

- Type: String
- Description: The LLM provider to use. Can be either "openai" or "claude".
- Example: `llm_provider = "openai"`

### `use_gitmoji`

- Type: Boolean
- Description: Whether to include gitmoji in commit messages.
- Example: `use_gitmoji = true`

### `custom_instructions`

- Type: String
- Description: Custom instructions to be included in the prompt for the LLM.
- Example: `custom_instructions = "Always mention the ticket number if applicable."`

## Example Configuration File

```toml
api_key = "sk-1234567890abcdef"
llm_provider = "openai"
use_gitmoji = true
custom_instructions = """
Always mention the ticket number if applicable.
Use imperative mood in the commit message.
"""
```

## Changing Configuration

You can change the configuration using the `git-iris config` command:

```
git-iris config --api-key YOUR_API_KEY
git-iris config --llm-provider openai
git-iris config --gitmoji true
git-iris config --custom-instructions "Your custom instructions here"
```

Alternatively, you can edit the `~/.git-iris` file directly using a text editor.