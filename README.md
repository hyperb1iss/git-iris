# 🔮 Git-Iris: AI-Powered Commit Messages

<div align="center">

[![CI/CD](https://github.com/hyperb1iss/git-iris/actions/workflows/cicd.yml/badge.svg)](https://github.com/hyperb1iss/git-iris/actions)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![GitHub Release][releases-shield]][releases]

*Elevate your Git commit messages with the power of AI* 🚀

[Installation](#installation) • [Configuration](#configuration) • [Usage](#usage) • [Contributing](#contributing) • [License](#license)

</div>

<div align="center">
  <img src="https://raw.githubusercontent.com/hyperb1iss/git-iris/main/docs/images/git-iris-screenshot-1.png" alt="Git-Iris Screenshot 1" width="33%">
  <img src="https://raw.githubusercontent.com/hyperb1iss/git-iris/main/docs/images/git-iris-screenshot-2.png" alt="Git-Iris Screenshot 2" width="33%">
  <img src="https://raw.githubusercontent.com/hyperb1iss/git-iris/main/docs/images/git-iris-screenshot-3.png" alt="Git-Iris Screenshot 3" width="33%">
</div>

*Git-Iris in action: AI-powered commit message generation and interactive refinement*

## ✨ Features

- 🤖 **AI-powered commit message generation** using state-of-the-art language models
- 📜 **Changelog generation** for tracking project history
- 📋 **Release notes generation** for creating comprehensive summaries of changes
- 🔄 **Multi-provider support** (OpenAI GPT-4o, Anthropic Claude, Ollama)
- 🎨 **Gitmoji integration** for expressive commit messages
- 🖥️ **Interactive CLI** for reviewing and refining AI-generated messages (*vibes included*)
- 🔧 **Customizable prompts and instructions** to tailor AI output
- 📚 **Flexible instruction presets** for quick, consistent, and customizable commit styles
- 🧠 **Smart context extraction** from Git repositories
- 📊 **Intelligent code change analysis** for context-aware suggestions
- 🔍 **Relevance scoring** to prioritize important changes
- 📝 **Support for multiple programming languages** including Rust, JavaScript, Python, Java, and more
- 🚀 **Optimized for performance** with efficient token management

## 🛠️ Installation

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

## ⚙️ Configuration

Git-Iris uses a configuration file located at `~/.config/git-iris/config.toml`. Set up your preferred AI provider:

```bash
# For OpenAI
git-iris config --provider openai --api-key YOUR_OPENAI_API_KEY

# For Anthropic Claude
git-iris config --provider claude --api-key YOUR_CLAUDE_API_KEY

# For Ollama (no API key required)
git-iris config --provider ollama
```

Additional configuration options:

```bash
# Set default provider
git-iris config --default-provider openai

# Enable/Disable Gitmoji
git-iris config --gitmoji true

# Set custom instructions
git-iris config --instructions "Always mention the ticket number in the commit message"

# Set default instruction preset
git-iris config --preset conventional

# Set token limit for a provider
git-iris config --provider openai --token-limit 4000

# Set model for a provider
git-iris config --provider openai --model gpt-4o

# Set additional parameters for a provider
git-iris config --provider openai --param temperature=0.7 --param max_tokens=150
```

For more detailed configuration information, please refer to our [Configuration Guide](CONFIG.md).

## 📖 Usage

Generate an AI-powered commit message:

```bash
git-iris gen
```

Options:
- `-a`, `--auto-commit`: Automatically commit with the generated message
- `-i`, `--instructions`: Provide custom instructions for this commit
- `--provider`: Specify an LLM provider (openai, claude, ollama)
- `--preset`: Use a specific instruction preset
- `--no-gitmoji`: Disable Gitmoji for this commit
- `-l`, `--log`: Enable logging to file
- `-p`, `--print`: Print the generated message to stdout and exit

Example:
```bash
git-iris gen -a -i "Focus on performance improvements" --provider claude --preset detailed
```

To generate a commit message and print it to stdout without starting the interactive process:

```bash
git-iris gen --print
```

### Interactive Commit Process

The interactive CLI allows you to refine and perfect your commit messages:

- Use arrow keys to navigate through suggestions
- Press 'e' to edit the current message
- Press 'i' to modify AI instructions
- Press 'r' to regenerate the message
- Press Enter to commit
- Press Esc to cancel

### Generating a Changelog

Git-Iris can generate changelogs between two Git references:

```bash
git-iris changelog --from <from-ref> --to <to-ref>
```

Options:
- `--from`: Starting Git reference (commit hash, tag, or branch name)
- `--to`: Ending Git reference (defaults to HEAD if not specified)
- `--instructions`: Custom instructions for changelog generation
- `--preset`: Select an instruction preset for changelog generation
- `--detail-level`: Set the detail level (minimal, standard, detailed)
- `--gitmoji`: Enable or disable Gitmoji in the changelog

Example:
```bash
git-iris changelog --from v1.0.0 --to v1.1.0 --detail-level detailed --gitmoji true
```

This command generates a detailed changelog of changes between versions 1.0.0 and 1.1.0, including Gitmoji.

### Generating Release Notes

Git-Iris can also generate comprehensive release notes:

```bash
git-iris release-notes --from <from-ref> --to <to-ref>
```

Options:
- `--from`: Starting Git reference (commit hash, tag, or branch name)
- `--to`: Ending Git reference (defaults to HEAD if not specified)
- `--instructions`: Custom instructions for release notes generation
- `--preset`: Select an instruction preset for release notes generation
- `--detail-level`: Set the detail level (minimal, standard, detailed)
- `--gitmoji`: Enable or disable Gitmoji in the release notes

Example:
```bash
git-iris release-notes --from v1.0.0 --to v1.1.0 --preset conventional --detail-level standard
```

This command generates standard-level release notes between versions 1.0.0 and 1.1.0 using the conventional commits preset.

## 🎛️ Custom Instructions and Presets

Git-Iris offers two powerful ways to guide the AI in generating commit messages: custom instructions and presets.

### Instruction Presets

Presets are predefined sets of instructions that provide a quick way to adjust the commit message style. Git-Iris comes with several built-in presets to suit different commit styles and project needs.

To list available presets:

```bash
git-iris list-presets
```

This will display a list of all available presets with a brief description of each.

To view details of a specific preset:

```bash
git-iris show-preset conventional
```

This will show you the full instructions associated with the 'conventional' preset.

Some key presets include:

- `default`: Standard commit message style
- `conventional`: Follows the Conventional Commits specification
- `detailed`: Provides more context and explanation in commit messages
- `concise`: Short and to-the-point commit messages
- `cosmic`: Mystical, space-themed commit messages
- ..and lots more styles f

To use a preset for a single commit:

```bash
git-iris gen --preset conventional
```

To set a default preset for all commits:

```bash
git-iris config --preset conventional
```

Presets work seamlessly with other Git-Iris features. For example, if you have Gitmoji enabled, the preset instructions will be applied in addition to adding the appropriate Gitmoji.

### Custom Instructions

Custom instructions allow you to provide specific guidance for commit message generation. These can be set globally or per-commit.

Setting global custom instructions:
```bash
git-iris config --instructions "Always include the ticket number and mention performance impacts"
```

Providing per-commit instructions:
```bash
git-iris gen -i "Emphasize security implications of this change"
```

### Combining Presets and Custom Instructions

When using both a preset and custom instructions, Git-Iris combines them, with custom instructions taking precedence. This allows you to use a preset as a base and fine-tune it with specific instructions.

```bash
git-iris gen --preset conventional -i "Mention the JIRA ticket number"
```

In this case, the commit message will follow the Conventional Commits format and include the JIRA ticket number.

If you've set a default preset in your configuration, you can still override it for individual commits:

```bash
git-iris gen --preset detailed -i "Focus on performance improvements"
```

This will use the 'detailed' preset instead of your default, along with the custom instruction.

### Examples of Custom Instructions

1. **Ticket Number Integration**
   ```
   Always start the commit message with the JIRA ticket number in square brackets
   ```

2. **Language-Specific Conventions**
   ```
   For Rust files, mention any changes to public APIs or use of unsafe code
   ```

3. **Team-Specific Guidelines**
   ```
   Follow the Angular commit message format: <type>(<scope>): <subject>
   ```

4. **Project-Specific Context**
   ```
   For the authentication module, always mention if there are changes to the user model or permissions
   ```

5. **Performance Considerations**
   ```
   Highlight any changes that might affect application performance, including database queries
   ```

Custom instructions and presets allow you to tailor Git-Iris to your specific project needs, team conventions, or personal preferences. They provide a powerful way to ensure consistency and capture important context in your commit messages.

## 🤝 Contributing

We welcome contributions! Please see our [CONTRIBUTING.md](CONTRIBUTING.md) for details on how to get started, our code of conduct, and the process for submitting pull requests.

## 📄 License

Distributed under the Apache 2.0 License. See `LICENSE` for more information.

---

<div align="center">

📚 [Documentation](https://github.com/hyperb1iss/git-iris/wiki) • 🐛 [Report Bug](https://github.com/hyperb1iss/git-iris/issues) • 💡 [Request Feature](https://github.com/hyperb1iss/git-iris/issues)

</div>

## 💖 Acknowledgements

- [OpenAI](https://openai.com/) and [Anthropic](https://www.anthropic.com/) for their cutting-edge language models
- The Rust community for the robust ecosystem and tooling

---

<div align="center">

Created by [Stefanie Jane 🌠](https://github.com/hyperb1iss)

If you find Git-Iris useful, consider [buying me a Monster Ultra Violet](https://ko-fi.com/hyperb1iss)! ⚡️

</div>

[crates-shield]: https://img.shields.io/crates/v/git-iris.svg
[crates]: https://crates.io/crates/git-iris
[releases-shield]: https://img.shields.io/github/release/hyperb1iss/git-iris.svg
[releases]: https://github.com/hyperb1iss/git-iris/releases
[license-shield]: https://img.shields.io/github/license/hyperb1iss/git-iris.svg