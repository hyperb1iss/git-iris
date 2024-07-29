
# 🔮 Git-Iris for Better Commit Messages

<div align="center">

[![Crates.io][crates-shield]][crates]
[![GitHub Release][releases-shield]][releases]
[![License][license-shield]](LICENSE)

*Elevate your Git commit messages with the power of AI* 🚀

[Installation](#installation) • [Configuration](#configuration) • [Usage](#usage) • [Contributing](#contributing) • [License](#license)

</div>

## ✨ Features

- 🤖 **AI-powered commit message generation**
- 🔄 **Support for multiple AI providers (OpenAI, Claude)**
- 🎨 **Optional Gitmoji integration**
- 🔧 **Customizable prompts and instructions**
- 📊 **Intelligent analysis of your code changes**
- 🖥️ **Interactive CLI for reviewing and editing suggestions**
- 🔐 **Secure handling of API keys**

## 🌈 Screenshots
<table>
  <tr>
    <td><img src="images/commit_generation.png" alt="Commit Generation"/></td>
    <td><img src="images/interactive_cli.png" alt="Interactive CLI"/></td>
  </tr>
  <tr>
    <td><img src="images/gitmoji_integration.png" alt="Gitmoji Integration"/></td>
    <td><img src="images/provider_configuration.png" alt="Provider Configuration"/></td>
  </tr>
</table>

## 🛠️ Installation
<a name="installation"></a>

### Prerequisites

- Rust and Cargo installed on your system
- Git 2.23.0 or newer

### Cargo Installation (Recommended)

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
<a name="configuration"></a>

Configure your preferred AI provider:

```bash
# For OpenAI
git-iris config --provider openai --api-key YOUR_OPENAI_API_KEY

# For Claude
git-iris config --provider claude --api-key YOUR_CLAUDE_API_KEY
```

Additional configuration options:

```bash
# Enable Gitmoji
git-iris config --gitmoji true

# Set custom instructions
git-iris config --custom-instructions "Please ensure all commit messages are professional."
```

## 📖 Usage
<a name="usage"></a>

Generate a commit message using AI:

```bash
git-iris gen
```

Options:
- `--verbose`: Enable verbose mode.
- `--gitmoji`: Override use_gitmoji setting.
- `--provider`: Override default LLM provider.

Example:
```bash
git-iris gen --verbose --gitmoji --provider openai
```

### Interactive Commit Process

The interactive commit process allows you to refine and finalize commit messages step-by-step. Use the arrow keys to navigate, 'e' to edit the message, 'i' to edit instructions, Enter to commit, and Esc to cancel.

### Gitmoji

If Gitmoji support is enabled, you can include Gitmojis in your commit messages. Gitmojis add visual representation to your commit messages, making them easier to understand at a glance.

## Examples

### Generating a Commit Message with Custom Instructions
```bash
git-iris gen --custom-instructions "Focus on the impact of the changes."
```

### Configuring OpenAI as the Default Provider
```bash
git-iris config --provider openai --api-key YOUR_API_KEY --model gpt-4
```

### Interactive Commit Process
```bash
git-iris gen
```

Navigate with arrow keys, edit messages and instructions, and finalize your commit.

## 🤝 Contributing
<a name="contributing"></a>

Contributions are what make the open-source community such a fantastic place to learn, inspire, and create. Any contributions you make are **greatly appreciated**. Please see our [CONTRIBUTING.md](CONTRIBUTING.md) file for more details on how to get started.

## 📄 License
<a name="license"></a>

Distributed under the Apache 2.0 License. See `LICENSE` for more information.

---

<div align="center">

📚 [Documentation](https://github.com/hyperb1iss/git-iris/wiki) • 🐛 [Report Bug](https://github.com/hyperb1iss/git-iris/issues) • 💡 [Request Feature](https://github.com/hyperb1iss/git-iris/issues)

</div>

## 💖 Acknowledgements

- [OpenAI](https://openai.com/) and [Anthropic](https://www.anthropic.com/) for their powerful language models
- The Rust community for the amazing ecosystem and tools

---

<div align="center">

Created by [Stefanie Jane 🌠](https://github.com/hyperb1iss)

If you find this project useful, [buy me a Monster Ultra Violet!](https://ko-fi.com/hyperb1iss)! ⚡️

</div>

[crates-shield]: https://img.shields.io/crates/v/git-iris.svg
[crates]: https://crates.io/crates/git-iris
[releases-shield]: https://img.shields.io/github/release/hyperb1iss/git-iris.svg
[releases]: https://github.com/hyperb1iss/git-iris/releases
[license-shield]: https://img.shields.io/github/license/hyperb1iss/git-iris.svg
