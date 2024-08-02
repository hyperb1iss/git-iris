# Git-Iris Usage Guide

## Table of Contents
1. [Introduction](#introduction)
2. [Installation](#installation)
3. [Configuration](#configuration)
4. [Basic Usage](#basic-usage)
5. [Advanced Features](#advanced-features)
6. [Best Practices](#best-practices)
7. [Troubleshooting](#troubleshooting)
8. [FAQ](#faq)

## 1. Introduction <a name="introduction"></a>

Git-Iris is an AI-powered tool designed to generate meaningful and context-aware Git commit messages. By analyzing your code changes and project context, Git-Iris provides high-quality commit messages that accurately describe your work.

## 2. Installation <a name="installation"></a>

### Prerequisites
- Rust and Cargo (latest stable version)
- Git 2.23.0 or newer

### Via Cargo (Recommended)
```bash
cargo install git-iris
```

### Manual Installation
1. Clone the repository:
   ```bash
   git clone https://github.com/hyperb1iss/git-iris.git
   cd git-iris
   ```
2. Build and install:
   ```bash
   cargo build --release
   cargo install --path .
   ```

## 3. Configuration <a name="configuration"></a>

Git-Iris uses a configuration file located at `~/.config/git-iris/config.toml`. You can set it up using the following commands:

```bash
# Set up OpenAI as the provider
git-iris config --provider openai --api-key YOUR_OPENAI_API_KEY

# Set up Claude as the provider
git-iris config --provider claude --api-key YOUR_CLAUDE_API_KEY

# Enable Gitmoji
git-iris config --gitmoji true

# Set custom instructions
git-iris config --custom-instructions "Always mention the ticket number in the commit message"
```

For more detailed configuration options, refer to the [Configuration Guide](CONFIG.md).

## 4. Basic Usage <a name="basic-usage"></a>

### Generating a Commit Message
1. Stage your changes using `git add`
2. Run the following command:
   ```bash
   git-iris gen
   ```
3. Review the generated message in the interactive interface
4. Accept, edit, or regenerate the message as needed
5. Confirm to create the commit

### Command-line Options
- `--verbose`: Enable detailed output
- `--gitmoji`: Override the Gitmoji setting
- `--provider`: Specify an LLM provider
- `--auto-commit`: Automatically commit with the generated message

Example:
```bash
git-iris gen --verbose --gitmoji --provider openai
```

## 5. Advanced Features <a name="advanced-features"></a>

### Custom Instructions
You can provide custom instructions to guide the AI in generating commit messages:

```bash
git-iris gen --custom-instructions "Focus on performance improvements and API changes"
```

### Interactive CLI Navigation
- Use arrow keys to navigate through suggestions
- Press 'e' to edit the current message
- Press 'i' to modify AI instructions
- Press 'r' to regenerate the message
- Press Enter to commit
- Press Esc to cancel

### Token Optimization
Git-Iris automatically optimizes token usage to stay within provider limits while maximizing context. You can set a custom token limit:

```bash
git-iris config --token-limit 8000
```

### Multiple LLM Providers
Git-Iris supports multiple LLM providers. You can switch between them:

```bash
git-iris gen --provider claude
```

## 6. Best Practices <a name="best-practices"></a>

1. **Stage Changes Carefully**: Only stage the changes you want to include in the commit before running Git-Iris.

2. **Review Generated Messages**: Always review and, if necessary, edit the AI-generated messages to ensure accuracy.

3. **Use Custom Instructions**: Tailor the AI output to your project's needs by setting appropriate custom instructions.

4. **Leverage Gitmoji**: Enable Gitmoji for more expressive and categorized commit messages.

5. **Combine with Conventional Commits**: Use custom instructions to guide Git-Iris in following the Conventional Commits format if your project requires it.

6. **Optimize for Performance**: For large repositories, consider using a higher token limit to provide more context to the AI.

## 7. Troubleshooting <a name="troubleshooting"></a>

### Issue: Git-Iris fails to generate a message
- Ensure your API key is correctly set in the configuration
- Check your internet connection
- Verify that you have staged changes in your repository

### Issue: Generated messages are not relevant
- Try providing more specific custom instructions
- Ensure you're using the latest version of Git-Iris
- Consider switching to a different LLM provider

### Issue: Token limit errors
- Increase the token limit in your configuration
- For very large changes, consider breaking them into smaller, logical commits

## 8. FAQ <a name="faq"></a>

**Q: Can I use Git-Iris with GitHub Actions or other CI/CD pipelines?**
A: While Git-Iris is primarily designed for local use, it can be integrated into CI/CD pipelines with some additional setup. Refer to our advanced documentation for details.

**Q: How does Git-Iris handle sensitive information?**
A: Git-Iris is designed to avoid sending sensitive information to LLM providers. However, always review generated messages to ensure no sensitive data is included.

**Q: Can I use Git-Iris for projects in languages it doesn't explicitly support?**
A: Yes, Git-Iris can generate commit messages for any text-based files. Language-specific analysis is available for supported languages, but basic analysis works for all text files.

**Q: How can I contribute to Git-Iris?**
A: We welcome contributions! Please refer to our [CONTRIBUTING.md](CONTRIBUTING.md) file for guidelines on how to contribute to the project.
