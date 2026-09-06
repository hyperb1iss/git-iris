# Troubleshooting Guide

Common issues and solutions for Git-Iris.

## Installation Issues

### Homebrew Installation Fails

**Problem:**

```
Error: git-iris not found in tap
```

**Solution:**

```bash
# Update Homebrew
brew update

# Tap the repository explicitly
brew tap hyperb1iss/tap

# Install
brew install git-iris
```

---

### Binary Not Found After Install

**Problem:**

```
command not found: git-iris
```

**Solution:**

```bash
# Check if installed
which git-iris

# If not in PATH, add Homebrew bin directory
echo 'export PATH="/usr/local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc

# Or use Homebrew's recommended PATH
echo 'eval "$(/opt/homebrew/bin/brew shellenv)"' >> ~/.zshrc
```

---

## Configuration Issues

### API Key Not Found

**Problem:**

```
Error: API key required for anthropic.
Set ANTHROPIC_API_KEY or configure in ~/.config/git-iris/config.toml
```

**Solutions:**

1. **Environment variable:**

   ```bash
   export ANTHROPIC_API_KEY="sk-ant-..."
   git-iris gen
   ```

2. **Config file:**

   ```bash
   git-iris config --provider anthropic --api-key YOUR_KEY
   ```

3. **Verify config:**
   ```bash
   cat ~/.config/git-iris/config.toml
   ```

---

### Wrong Provider Selected

**Problem:**
Git-Iris uses OpenAI when you want Anthropic.

**Solution:**

```bash
# Set default provider
git-iris config --provider anthropic

# Or override per-command
git-iris gen --provider anthropic
```

---

### Project Config Not Loading

**Problem:**
`.irisconfig` settings ignored.

**Solution:**

```bash
# Check file location (must be in repo root)
git rev-parse --show-toplevel
ls -la .irisconfig

# Check file format
cat .irisconfig

# Remember: API keys never load from .irisconfig
# Set them in personal config or environment
export ANTHROPIC_API_KEY="sk-ant-..."
```

---

### Invalid Model Name

**Problem:**

```
Error: Model 'claude-4' not found
```

**Solution:**

```bash
# Use correct model name
git-iris config --provider anthropic --model claude-opus-5

# List available providers
git-iris config --help

# Check provider documentation for model names
```

---

## Git Issues

### Not in Git Repository

**Problem:**

```
Not in a Git repository. Please run this command from within a Git repository.
```

**Solution:**

```bash
# Check if in Git repo
git status

# Initialize Git repo if needed
git init

# Or navigate to a Git repository
cd /path/to/your/repo
```

---

### No Staged Changes

**Problem:**

```
No staged changes. Please stage your changes before generating a commit message.
```

**Solution:**

```bash
# Stage specific files
git add file1.js file2.py

# Or stage all changes
git add .

# Then generate commit
git-iris gen
```

---

### Pre-Commit Hook Fails

**Problem:**

```
Pre-commit failed: <error message>
```

**Solutions:**

1. **Fix the issue:**

   ```bash
   # Address the pre-commit failure
   npm run lint
   git add .
   git-iris gen
   ```

2. **Skip hook (not recommended):**
   ```bash
   git-iris gen --no-verify --auto-commit
   ```

---

## Studio Issues

### Studio Won't Launch

**Problem:**
Studio crashes or shows garbled output.

**Solutions:**

1. **Check terminal true color support:**

   ```bash
   printf "\x1b[38;2;255;100;0mTRUECOLOR\x1b[0m\n"
   ```

2. **Update terminal:**
   - Use iTerm2, Alacritty, WezTerm, or Kitty
   - Update to latest version

3. **Try different theme:**

   ```bash
   git-iris studio --theme silkcircuit-soft
   ```

4. **Check terminal size:**
   ```bash
   # Studio requires minimum size
   tput cols  # Should be > 80
   tput lines # Should be > 24
   ```

---

### Studio Keybindings Not Working

**Problem:**
Pressing keys does nothing.

**Solution:**

```bash
# Check if in editing mode (press Esc first)
# Verify you're in the right panel (use Tab)
# Check help overlay (press ?)
```

---

### Can't See Colors in Studio

**Problem:**
No colors or wrong colors displayed.

**Solutions:**

1. **Enable true color in tmux:**

   ```bash
   # Add to ~/.tmux.conf
   set -g default-terminal "tmux-256color"
   set -ga terminal-overrides ",*256col*:Tc"
   ```

2. **Check TERM variable:**

   ```bash
   echo $TERM
   # Should be: xterm-256color or similar
   ```

3. **Try a different theme:**
   ```bash
   git-iris themes
   # Override per invocation
   git-iris studio --theme silkcircuit-dawn
   # Or make it permanent by adding `theme = "silkcircuit-dawn"` to
   # ~/.config/git-iris/config.toml (there is no `config --theme` flag).
   ```

---

## LLM Provider Issues

### Rate Limit Exceeded

**Problem:**

```
Error: Rate limit exceeded for provider
```

**Solutions:**

1. **Wait and retry:**

   ```bash
   # Wait a minute, then retry
   sleep 60
   git-iris gen
   ```

2. **Switch to different provider:**

   ```bash
   git-iris gen --provider openai
   ```

3. **Use fast model:**
   ```bash
   git-iris config --fast-model claude-haiku-4-5-20251001
   ```

---

### Authentication Failed

**Problem:**

```
Error: Authentication failed for provider
```

**Solutions:**

1. **Verify API key:**

   ```bash
   # Check environment variable
   echo $ANTHROPIC_API_KEY

   # Check config file
   cat ~/.config/git-iris/config.toml | grep api_key
   ```

2. **Regenerate API key:**
   - Visit provider dashboard
   - Create new API key
   - Update configuration
   ```bash
   git-iris config --provider anthropic --api-key NEW_KEY
   ```

---

### Token Limit Exceeded

**Problem:**

```
Error: Context length exceeded
```

**Solutions:**

1. **Reduce token limit:**

   ```bash
   git-iris config --token-limit 50000
   ```

2. **Use provider with larger context:**

   ```bash
   git-iris gen --provider google  # 1M context window
   ```

3. **Stage fewer files:**

   ```bash
   # Stage files incrementally
   git add src/
   git-iris gen --auto-commit

   git add tests/
   git-iris gen --auto-commit
   ```

---

### Slow Response Times

**Problem:**
Git-Iris takes too long to respond.

**Solutions:**

1. **Use fast model:**

   ```bash
   git-iris config --fast-model claude-haiku-4-5-20251001
   ```

2. **Reduce changeset size:**

   ```bash
   # Stage fewer files at once
   git add file1.js file2.js
   git-iris gen
   ```

3. **Switch provider:**
   ```bash
   # Try different provider
   git-iris gen --provider openai
   ```

> **Note on Anthropic prompt caching:** Git-Iris enables automatic prompt caching for Anthropic models via rig-core 0.37. The first call for a given system prompt is full price; subsequent calls within the cache TTL pay a fraction for the cached portion. This happens transparently — no configuration required.

---

## Debug and Logging

### Enable Debug Logging

```bash
# Basic logging
git-iris gen --log

# Custom log file
git-iris gen --log --log-file debug.log

# Color-coded agent execution
git-iris gen --debug

# View log
cat git-iris-debug.log
```

---

### Verbose Rust Logging

```bash
# Set Rust log level
export RUST_LOG=debug

# Run command
git-iris gen --log

# Check debug.log for details
```

---

## Performance Issues

### Large Repository Performance

**Problem:**
Git-Iris is slow in large repositories.

**Solutions:**

1. **Use .gitignore:**

   ```bash
   # Ignore build artifacts, node_modules, etc.
   echo "node_modules/" >> .gitignore
   echo "target/" >> .gitignore
   ```

2. **Stage selectively:**

   ```bash
   # Don't stage everything
   git add src/ tests/
   git-iris gen
   ```

3. **Use fast model:**
   ```bash
   git-iris config --fast-model claude-haiku-4-5-20251001
   ```

---

### Memory Issues

**Problem:**
Git-Iris crashes with out of memory.

**Solutions:**

1. **Reduce token limit:**

   ```bash
   git-iris config --token-limit 50000
   ```

2. **Stage fewer files:**
   ```bash
   git add --patch  # Stage selectively
   ```

---

## Network Issues

### Proxy Configuration

**Problem:**
Corporate proxy blocks API requests.

**Solution:**

```bash
# Set proxy environment variables
export HTTP_PROXY="http://proxy.company.com:8080"
export HTTPS_PROXY="http://proxy.company.com:8080"

# Run Git-Iris
git-iris gen
```

---

### Timeout Errors

**Problem:**

```
Error: Request timeout
```

**Solutions:**

1. **Check internet connection:**

   ```bash
   ping api.anthropic.com
   ```

2. **Retry:**

   ```bash
   git-iris gen
   ```

3. **Use different provider:**
   ```bash
   git-iris gen --provider openai
   ```

---

## Common Error Messages

### "No changes to commit"

**Cause:** No staged files.

**Fix:**

```bash
git add .
git-iris gen
```

---

### "Failed to parse response"

**Cause:** LLM returned invalid format.

**Fix:**

```bash
# Retry (usually resolves itself)
git-iris gen

# Or regenerate in Studio
git-iris studio --mode commit
# Press 'r' to regenerate
```

---

### "Invalid JSON in response"

**Cause:** LLM output parsing error.

**Fix:**

```bash
# Enable debug mode to see raw response
git-iris gen --debug

# Retry with different model
git-iris gen --provider openai
```

---

## Getting Help

### Report an Issue

1. **Collect information:**

   ```bash
   git-iris --version
   git-iris gen --log --log-file debug.log
   ```

2. **Create issue:**
   - Visit: https://github.com/hyperb1iss/git-iris/issues
   - Include version, error message, and debug.log

### Debug Checklist

Before reporting, verify:

- [ ] API key is set correctly
- [ ] You're in a Git repository
- [ ] Changes are staged
- [ ] Provider is configured
- [ ] Model name is correct
- [ ] Internet connection works
- [ ] Terminal supports true color

### Useful Commands

```bash
# Version info
git-iris --version

# Config check
cat ~/.config/git-iris/config.toml

# Git status
git status

# Environment check
env | grep API_KEY

# Test simple command
git-iris gen --print

# Full debug
git-iris gen --debug --log
```

---

## Advanced Troubleshooting

### Reset Configuration

```bash
# Backup current config
cp ~/.config/git-iris/config.toml ~/.config/git-iris/config.toml.backup

# Remove config
rm ~/.config/git-iris/config.toml

# Reconfigure
git-iris config --provider anthropic --api-key YOUR_KEY
```

---

### Clear Cache

```bash
# Git-Iris doesn't cache, but clear Git cache if needed
git rm -r --cached .
git add .
```

---

### Reinstall

```bash
# Uninstall
brew uninstall git-iris

# Clean cache
brew cleanup

# Reinstall
brew install hyperb1iss/tap/git-iris
```

---

## Platform-Specific Issues

### macOS

**Problem:** "git-iris" cannot be opened because the developer cannot be verified.

**Solution:**

```bash
# Allow binary
sudo spctl --add /usr/local/bin/git-iris

# Or in System Preferences:
# Security & Privacy → General → Allow
```

---

### Linux

**Problem:** Permission denied.

**Solution:**

```bash
chmod +x /usr/local/bin/git-iris
```

---

### Windows (WSL)

**Problem:** Colors not working.

**Solution:**

```bash
# Use Windows Terminal
# Or set TERM variable
export TERM=xterm-256color
```
