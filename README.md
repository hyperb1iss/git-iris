# 🔮 Git-Iris: AI-Powered Commit Messages

<div align="center">

[![Crates.io][crates-shield]][crates]
[![GitHub Release][releases-shield]][releases]
[![License][license-shield]](LICENSE)

*Elevate your Git commit messages with the power of AI* 🚀

[Installation](#installation) • [Configuration](#configuration) • [Usage](#usage) • [Contributing](#contributing) • [License](#license)

</div>

## ✨ Features

- 🤖 **AI-powered commit message generation** using state-of-the-art language models
- 🔄 **Multi-provider support** (OpenAI GPT-4o, Anthropic Claude)
- 🎨 **Gitmoji integration** for expressive commit messages (enabled by default)
- 🔧 **Customizable prompts and instructions** to tailor AI output
- 📊 **Intelligent code change analysis** for context-aware suggestions
- 🖥️ **Interactive CLI** for reviewing and refining AI-generated messages
- 🔐 **Secure API key management**
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

Git-Iris uses a configuration file located at `~/.config/git-iris/config.toml`. You can set up your preferred AI provider using the following commands:

```bash
# For OpenAI
git-iris config --provider openai --api-key YOUR_OPENAI_API_KEY

# For Anthropic Claude
git-iris config --provider claude --api-key YOUR_CLAUDE_API_KEY
```

Additional configuration options:

```bash
# Disable Gitmoji (enabled by default)
git-iris config --gitmoji false

# Set instructions
git-iris config --instructions "Ensure all commit messages are concise and descriptive."

# Set token limit (example for 5000 tokens)
git-iris config --token-limit 5000
```

For more detailed configuration information, please refer to our [Configuration Guide](CONFIG.md).

## 📖 Usage

Generate an AI-powered commit message:

```bash
git-iris gen
```

Options:
- `-l`, `--log`: Enable logging to file
- `-a`, `--auto-commit`: Automatically commit with the generated message
- `-i`, `--instructions`: Provide custom instructions for this commit
- `--provider`: Specify an LLM provider
- `--no-gitmoji`: Disable Gitmoji for this commit

Example:
```bash
git-iris gen -a -i "Focus on performance improvements" --provider openai
```

### Interactive Commit Process

The interactive CLI allows you to refine and perfect your commit messages:

- Use arrow keys to navigate through suggestions
- Press 'e' to edit the current message
- Press 'i' to modify AI instructions
- Press 'r' to regenerate the message
- Press Enter to commit
- Press Esc to cancel

### Gitmoji Support

Gitmoji is enabled by default, adding visual flair to your commit messages and making them more expressive and easier to categorize at a glance. Use the `--no-gitmoji` flag to disable it for a specific commit.

For more detailed usage information and advanced features, please refer to our [Usage Guide](USAGE.md).

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