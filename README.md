# 🔮 Git-Iris: AI-Powered Commit Messages

<div align="center">

[![CI/CD](https://github.com/hyperb1iss/git-iris/actions/workflows/cicd.yml/badge.svg)](https://github.com/hyperb1iss/git-iris/actions)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![GitHub Release][releases-shield]][releases]

*Elevate your Git commit messages with the power of AI* 🚀

[Installation](#installation) • [Configuration](#configuration) • [Usage](#usage) • [Contributing](#contributing) • [License](#license)

</div>

<div align="center">
  <img src="https://raw.githubusercontent.com/hyperb1iss/git-iris/main/docs/images/git-iris-screenshot-1.png" alt="Git-Iris Screenshot 1" width="48%">
  <img src="https://raw.githubusercontent.com/hyperb1iss/git-iris/main/docs/images/git-iris-screenshot-2.png" alt="Git-Iris Screenshot 2" width="48%">
</div>

*Git-Iris in action: AI-powered commit message generation and interactive refinement*

## ✨ Features

- 🤖 **AI-powered commit message generation** using state-of-the-art language models
- 🔄 **Multi-provider support** (OpenAI GPT-4o, Anthropic Claude, Ollama)
- 🎨 **Gitmoji integration** for expressive commit messages
- 🖥️ **Interactive CLI** for reviewing and refining AI-generated messages (*vibes included*)
- 🔧 **Customizable prompts and instructions** to tailor AI output
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
- `--no-gitmoji`: Disable Gitmoji for this commit
- `-l`, `--log`: Enable logging to file

Example:
```bash
git-iris gen -a -i "Focus on performance improvements" --provider claude
```

### Interactive Commit Process

The interactive CLI allows you to refine and perfect your commit messages:

- Use arrow keys to navigate through suggestions
- Press 'e' to edit the current message
- Press 'i' to modify AI instructions
- Press 'r' to regenerate the message
- Press Enter to commit
- Press Esc to cancel


## 🎛️ Custom Instructions

Git-Iris allows you to provide custom instructions to guide the AI in generating commit messages. These instructions can be set globally in the configuration or provided on a per-commit basis.

### Setting Global Custom Instructions

```bash
git-iris config --instructions "Always include the ticket number and mention performance impacts"
```

### Providing Per-Commit Instructions

```bash
git-iris gen -i "Emphasize security implications of this change"
```

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

6. **Documentation Updates**
   ```
   If documentation files are changed, start the commit message with 'docs:'
   ```

7. **Breaking Changes**
   ```
   Clearly indicate any breaking changes in the commit message, starting with 'BREAKING CHANGE:'
   ```

Custom instructions allow you to tailor Git-Iris to your specific project needs, team conventions, or personal preferences. They provide a powerful way to ensure consistency and capture important context in your commit messages.

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