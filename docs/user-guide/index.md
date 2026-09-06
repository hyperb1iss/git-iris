# User Guide

Git-Iris is an AI-powered Git workflow assistant that uses Iris, an intelligent agent built on the Rig framework, to help you write commit messages, review code, generate PR descriptions, and maintain changelogs.

## Core Capabilities

| Feature             | Command                  | What It Does                                                    |
| ------------------- | ------------------------ | --------------------------------------------------------------- |
| **Commit Messages** | `git-iris gen`           | Generates conventional commit messages with optional emoji      |
| **Code Reviews**    | `git-iris review`        | Performs structured code analysis with severity-graded findings |
| **PR Descriptions** | `git-iris pr`            | Creates detailed pull request descriptions                      |
| **Changelogs**      | `git-iris changelog`     | Generates Keep a Changelog-formatted entries                    |
| **Release Notes**   | `git-iris release-notes` | Creates release documentation                                   |
| **Studio TUI**      | `git-iris studio`        | Unified terminal interface for all operations                   |

## Quick Start

```bash
# Generate a commit message for staged changes
git add .
git-iris gen

# Auto-commit with generated message
git-iris gen --auto-commit

# Review staged changes
git-iris review

# Generate PR description comparing branches
git-iris pr --from main --to feature-branch

# Create changelog between versions
git-iris changelog --from v1.0.0 --to v2.0.0
```

## Global Flags

Available across all commands:

| Flag                        | Description                                                   |
| --------------------------- | ------------------------------------------------------------- |
| `--provider <name>`         | Override LLM provider (openai, anthropic, google)             |
| `--model <name>`            | Override model for this operation                             |
| `-r, --repo <url>`          | Run against a remote repository URL instead of the local repo |
| `-i, --instructions "text"` | Add custom instructions for this operation                    |
| `--preset <name>`           | Use instruction preset (see [Presets](./presets.md))          |
| `--critic` / `--no-critic`  | Run or skip the critic verification pass; commits opt in      |
| `--debug`                   | Enable color-coded agent execution visualization              |
| `--quiet`                   | Suppress non-essential output                                 |
| `--theme <name>`            | Override theme for this session                               |

## Configuration

Set your API key and preferences:

```bash
# Configure provider
git-iris config --provider openai --api-key YOUR_OPENAI_API_KEY

# Set default model
git-iris config --provider anthropic --model claude-opus-5

# Google works the same way
git-iris config --provider google --api-key YOUR_GOOGLE_API_KEY

# List available presets
git-iris list-presets

# List available themes
git-iris themes
```

## Next Steps

- [Commit Messages](./commits.md) - Generate intelligent commit messages
- [Code Reviews](./reviews.md) - AI-powered code analysis
- [Pull Requests](./pull-requests.md) - Professional PR descriptions
- [Changelogs](./changelogs.md) - Structured changelog generation
- [Release Notes](./release-notes.md) - Release documentation
- [Presets](./presets.md) - Customize Iris's behavior

## How Iris Works

Iris is an LLM-driven agent that dynamically explores your codebase using tool calls. Instead of dumping all context upfront, she:

1. **Analyzes** your request and repository state
2. **Calls tools** to gather precise information (git diff, file content, commit history)
3. **Adapts strategy** based on changeset size (full context, relevance scoring, or parallel analysis)
4. **Generates** structured output tailored to your workflow

This tool-based approach ensures Iris sees exactly what she needs without overwhelming her context window.
