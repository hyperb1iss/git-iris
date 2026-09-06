# CLI Command Reference

Complete reference for all Git-Iris commands and flags.

## Global Flags

Available on all commands:

| Flag                | Short | Description                                          |
| ------------------- | ----- | ---------------------------------------------------- |
| `--log`             | `-l`  | Log debug messages to file                           |
| `--log-file <PATH>` |       | Custom log file path (default: `git-iris-debug.log`) |
| `--quiet`           | `-q`  | Suppress non-essential output                        |
| `--version`         | `-v`  | Display version information                          |
| `--repo <URL>`      | `-r`  | Use remote repository instead of local               |
| `--debug`           |       | Enable debug mode with color-coded agent execution   |
| `--theme <NAME>`    |       | Override theme for this session                      |
| `--help`            | `-h`  | Show help information                                |

## Shared Flags (CommonParams)

Every feature command (`gen`, `review`, `pr`, `changelog`, `release-notes`) accepts the same set of LLM/repository flags, in addition to the command-specific ones documented below:

| Flag                    | Short | Description                                                      |
| ----------------------- | ----- | ---------------------------------------------------------------- |
| `--provider <NAME>`     |       | Override default provider for this invocation                    |
| `--model <NAME>`        |       | Override model for this invocation                               |
| `--instructions <TEXT>` | `-i`  | Custom instructions                                              |
| `--preset <NAME>`       |       | Instruction preset name (see `git-iris list-presets`)            |
| `--gitmoji`             |       | Enable gitmoji for this invocation (mutually exclusive)          |
| `--no-gitmoji`          |       | Disable gitmoji for this invocation (mutually exclusive)         |
| `--critic`              |       | Enable the critic verification pass                              |
| `--no-critic`           |       | Disable the critic verification pass (mutually exclusive)        |
| `--repo <URL>`          | `-r`  | Operate on a remote repository URL instead of the local checkout |

These are surfaced again in each command's Options table only when behavior is unusual; otherwise assume the full set is available.

> **About the critic:** when enabled, Iris runs a verification + revision pass after the initial generation, catching factual errors and tightening the output before it's returned. It costs a second model call but materially improves quality. Reviews, PR descriptions, changelogs, and release notes use it by default; commit messages skip it unless you pass `gen --critic`.

## Commands

### `gen` - Generate Commit Messages

```bash
git-iris gen [OPTIONS]
```

Generate AI-powered commit messages for staged changes.

**Options:**

| Flag                    | Short | Description                                   |
| ----------------------- | ----- | --------------------------------------------- |
| `--auto-commit`         | `-a`  | Automatically commit with generated message   |
| `--amend`               |       | Amend the previous commit with staged changes |
| `--no-gitmoji`          |       | Disable gitmoji for this commit               |
| `--print`               | `-p`  | Print message to stdout and exit              |
| `--no-verify`           |       | Skip pre/post commit hooks                    |
| `--provider <NAME>`     |       | Override default provider                     |
| `--model <NAME>`        |       | Override model for this operation             |
| `--instructions <TEXT>` | `-i`  | Custom instructions                           |
| `--preset <NAME>`       |       | Instruction preset name                       |
| `--gitmoji`             |       | Enable gitmoji for this invocation            |
| `--critic`              |       | Opt into critic verification for this commit  |
| `--no-critic`           |       | Keep critic verification disabled             |

**Examples:**

```bash
# Interactive mode (launches Studio)
git-iris gen

# Print only
git-iris gen --print

# Auto-commit
git-iris gen --auto-commit

# Use specific provider
git-iris gen --provider google --print

# Custom instructions
git-iris gen -i "Focus on security implications" --print
```

---

### `studio` - Launch Iris Studio

```bash
git-iris studio [OPTIONS]
```

Launch unified TUI for all operations.

**Options:**

| Flag            | Description                                                                     |
| --------------- | ------------------------------------------------------------------------------- |
| `--mode <MODE>` | Initial mode: `explore`, `commit`, `review`, `pr`, `changelog`, `release-notes` |
| `--from <REF>`  | Starting ref for comparison                                                     |
| `--to <REF>`    | Ending ref for comparison                                                       |

Unknown `--mode` values print a warning and fall back to auto-detect rather than erroring out.

**Examples:**

```bash
# Auto-detect mode
git-iris studio

# Start in commit mode
git-iris studio --mode commit

# Start in PR mode with refs
git-iris studio --mode pr --from main --to feature-branch

# Start in release notes mode
git-iris studio --mode release-notes
```

---

### `review` - Code Review

```bash
git-iris review [OPTIONS]
```

Generate multi-dimensional code reviews with AI.

**Options:**

| Flag                            | Short | Description                                                |
| ------------------------------- | ----- | ---------------------------------------------------------- |
| `--print`                       | `-p`  | Print review to stdout                                     |
| `--raw`                         |       | Output raw markdown without formatting                     |
| `--include-unstaged`            |       | Include unstaged changes                                   |
| `--commit <HASH>`               |       | Review specific commit                                     |
| `--from <REF>`                  |       | Starting branch for comparison                             |
| `--to <REF>`                    |       | Target branch for comparison (alone, compares from `main`) |
| `--github-review`               |       | Publish review as a GitHub PR review comment               |
| `--pr <NUMBER>`                 |       | Target a specific GitHub pull request number               |
| `--github-inline-comments`      |       | Add validated inline comments for findings in the PR diff  |
| `--github-review-event <EVENT>` |       | `comment` (default), `request-changes`, or `approve`       |
| `--critic`                      |       | Enable critic verification (default: on)                   |
| `--no-critic`                   |       | Disable critic verification for this run                   |

Plus all [shared flags](#shared-flags-commonparams) — `--provider`, `--model`, `--instructions/-i`, `--preset`, `--gitmoji`/`--no-gitmoji`, `--repo/-r`.

When `--github-review` is set, validated structured findings (with file/line locations) publish as inline review comments in the target PR; `--github-inline-comments` also opens individual line threads for each finding.

**Examples:**

```bash
# Review staged changes
git-iris review

# Review specific commit
git-iris review --commit abc1234

# Review branch comparison
git-iris review --from main --to feature-branch

# Review everything from main to a target ref
git-iris review --to feature-branch

# Include unstaged changes
git-iris review --include-unstaged --print

# Publish review to the open GitHub PR for the current branch
git-iris review --github-review

# Target a specific PR with inline comments and request-changes
git-iris review --github-review --pr 123 \
  --github-inline-comments \
  --github-review-event request-changes
```

---

### `pr` - Pull Request Descriptions

```bash
git-iris pr [OPTIONS]
```

Generate pull request descriptions.

**Options:**

| Flag            | Short | Description                                                                                         |
| --------------- | ----- | --------------------------------------------------------------------------------------------------- |
| `--print`       | `-p`  | Print to stdout                                                                                     |
| `--raw`         |       | Output raw markdown                                                                                 |
| `--copy`        | `-c`  | Copy raw markdown to clipboard                                                                      |
| `--from <REF>`  |       | Starting ref (default: `main`)                                                                      |
| `--to <REF>`    |       | Target ref (default: `HEAD`)                                                                        |
| `--update`      |       | Update the GitHub PR body (revises existing text, adapts to PR templates). Alias: `--github-update` |
| `--pr <NUMBER>` |       | Target a specific GitHub pull request number when updating                                          |
| `--critic`      |       | Enable critic verification (default: on)                                                            |
| `--no-critic`   |       | Disable critic verification for this run                                                            |

Plus all [shared flags](#shared-flags-commonparams) — `--provider`, `--model`, `--instructions/-i`, `--preset`, `--gitmoji`/`--no-gitmoji`, `--repo/-r`.

**Examples:**

```bash
# PR from main to current branch
git-iris pr

# PR from specific branch
git-iris pr --from develop --to feature-branch

# Single commit PR
git-iris pr --from abc1234

# Copy markdown to clipboard
git-iris pr --copy

# Print only
git-iris pr --print

# Update the GitHub PR body for the current branch
git-iris pr --update

# Update a specific PR number with explicit refs
git-iris pr --from main --to feature-branch --update --pr 123
```

---

### `changelog` - Generate Changelog

```bash
git-iris changelog --from <REF> [OPTIONS]
```

Generate changelog between Git references.

**Options:**

| Flag                    | Required | Description                                   |
| ----------------------- | -------- | --------------------------------------------- |
| `--from <REF>`          | Yes      | Starting Git reference                        |
| `--to <REF>`            | No       | Ending reference (default: `HEAD`)            |
| `--raw`                 | No       | Output raw markdown                           |
| `--update`              | No       | Update CHANGELOG.md file                      |
| `--file <PATH>`         | No       | Changelog file path (default: `CHANGELOG.md`) |
| `--version-name <NAME>` | No       | Explicit version name                         |
| `--critic`              | No       | Enable critic verification (default: on)      |
| `--no-critic`           | No       | Disable critic verification for this run      |

Plus all [shared flags](#shared-flags-commonparams) — `--provider`, `--model`, `--instructions/-i`, `--preset`, `--gitmoji`/`--no-gitmoji`, `--repo/-r`.

**Examples:**

```bash
# Changelog from tag to HEAD
git-iris changelog --from v1.0.0

# Changelog between tags
git-iris changelog --from v1.0.0 --to v2.0.0

# Update CHANGELOG.md
git-iris changelog --from v1.0.0 --update

# Custom version name
git-iris changelog --from v1.0.0 --version-name "v2.0.0"
```

---

### `release-notes` - Generate Release Notes

```bash
git-iris release-notes --from <REF> [OPTIONS]
```

Generate detailed release notes.

**Options:**

| Flag                    | Required | Description                              |
| ----------------------- | -------- | ---------------------------------------- |
| `--from <REF>`          | Yes      | Starting Git reference                   |
| `--to <REF>`            | No       | Ending reference (default: `HEAD`)       |
| `--raw`                 | No       | Output raw markdown                      |
| `--update`              | No       | Update the release notes file            |
| `--file <PATH>`         | No       | Release notes file path                  |
| `--version-name <NAME>` | No       | Explicit version name                    |
| `--critic`              | No       | Enable critic verification (default: on) |
| `--no-critic`           | No       | Disable critic verification for this run |

Plus all [shared flags](#shared-flags-commonparams) — `--provider`, `--model`, `--instructions/-i`, `--preset`, `--gitmoji`/`--no-gitmoji`, `--repo/-r`.

**Examples:**

```bash
# Release notes from tag
git-iris release-notes --from v1.0.0

# Between tags
git-iris release-notes --from v1.0.0 --to v2.0.0

# Update RELEASE_NOTES.md
git-iris release-notes --from v1.0.0 --update

# Custom version
git-iris release-notes --from v1.0.0 --version-name "2.0.0-beta"
```

---

### `config` - Configuration Management

```bash
git-iris config [OPTIONS]
```

Configure global Git-Iris settings.

**Options:**

| Flag                           | Description                                    |
| ------------------------------ | ---------------------------------------------- |
| `--instructions <TEXT>`        | Set default instructions across capabilities                    |
| `--preset <NAME>`              | Set default preset                             |
| `--gitmoji`                    | Enable gitmoji                                 |
| `--no-gitmoji`                 | Disable gitmoji                                |
| `--critic`                     | Enable critic verification                     |
| `--no-critic`                  | Disable critic verification                    |
| `--provider <NAME>`            | Set default provider                           |
| `--api-key <KEY>`              | Set API key                                    |
| `--model <NAME>`               | Set primary model                              |
| `--fast-model <NAME>`          | Set fast model                                 |
| `--subagent-model <MODEL>`     | Set delegated analysis model                   |
| `--token-limit <NUM>`          | Set context-window metadata                    |
| `--param <KEY=VALUE>`          | Set additional parameters                      |
| `--subagent-timeout <SECONDS>` | Set parallel subagent timeout (default: `120`) |
| `--subagent-max-turns <NUM>`   | Set subagent turn budget (default: `20`)       |

**Examples:**

```bash
# Set provider and API key
git-iris config --provider openai --api-key sk-...

# Configure models
git-iris config --provider anthropic \
  --model claude-opus-5 \
  --fast-model claude-haiku-4-5-20251001

# Set token limit
git-iris config --provider openai --token-limit 8000

# Additional parameters
git-iris config --provider openai \
  --param reasoning='{"effort":"medium"}' \
  --param text='{"verbosity":"low"}'
```

---

### `project-config` - Project Configuration

```bash
git-iris project-config [OPTIONS]
```

Manage project-specific `.irisconfig` file.

**Options:**

| Flag                           | Short | Description                                    |
| ------------------------------ | ----- | ---------------------------------------------- |
| `--provider <NAME>`            |       | Set project provider                           |
| `--instructions <TEXT>`        |       | Set project instructions across capabilities                    |
| `--preset <NAME>`              |       | Set project preset                             |
| `--gitmoji`                    |       | Enable gitmoji                                 |
| `--no-gitmoji`                 |       | Disable gitmoji                                |
| `--critic`                     |       | Enable critic verification                     |
| `--no-critic`                  |       | Disable critic verification                    |
| `--model <NAME>`               |       | Set project model                              |
| `--fast-model <NAME>`          |       | Set project fast model                         |
| `--token-limit <NUM>`          |       | Set project context-window metadata            |
| `--param <KEY=VALUE>`          |       | Set project parameters                         |
| `--subagent-timeout <SECONDS>` |       | Set parallel subagent timeout (default: `120`) |
| `--subagent-max-turns <NUM>`   |       | Set subagent turn budget (default: `20`)       |
| `--print`                      | `-p`  | Print current project config                   |

**Examples:**

```bash
# Create project config
git-iris project-config --provider google

# Set project model
git-iris project-config --model gemini-3.8-flash

# View project config
git-iris project-config --print
```

---

### `list-presets` - List Instruction Presets

```bash
git-iris list-presets
```

Display all available instruction presets.

**No options.**

---

### `themes` - List Themes

```bash
git-iris themes
```

Display all available themes.

**No options.**

---

### `completions` - Generate Shell Completions

```bash
git-iris completions <SHELL>
```

Generate shell completion scripts.

**Arguments:**

| Argument     | Description                     |
| ------------ | ------------------------------- |
| `bash`       | Generate Bash completions       |
| `zsh`        | Generate Zsh completions        |
| `fish`       | Generate Fish completions       |
| `elvish`     | Generate Elvish completions     |
| `powershell` | Generate PowerShell completions |

**Examples:**

```bash
git-iris completions zsh >> ~/.zshrc
git-iris completions fish > ~/.config/fish/completions/git-iris.fish
```

---

### `hook` - Manage Git Hooks

```bash
git-iris hook <install|uninstall> [OPTIONS]
```

Install or uninstall the `prepare-commit-msg` hook.

**Subcommands:**

| Subcommand  | Description                         |
| ----------- | ----------------------------------- |
| `install`   | Install the prepare-commit-msg hook |
| `uninstall` | Remove the prepare-commit-msg hook  |

**Options for `install`:**

| Flag      | Description                             |
| --------- | --------------------------------------- |
| `--force` | Overwrite an existing non-git-iris hook |

**Examples:**

```bash
git-iris hook install
git-iris hook install --force
git-iris hook uninstall
```

## Common Workflows

### First-Time Setup

```bash
# Install
brew install hyperb1iss/tap/git-iris

# Configure
git-iris config --provider openai --api-key YOUR_OPENAI_API_KEY
git-iris config --provider anthropic --api-key YOUR_ANTHROPIC_API_KEY
git-iris config --provider google --api-key YOUR_GOOGLE_API_KEY
```

### Daily Usage

```bash
# Stage changes
git add .

# Generate commit (interactive)
git-iris gen

# Or auto-commit
git-iris gen --auto-commit
```

### Code Review Workflow

```bash
# Review staged changes
git add .
git-iris review

# Or review a PR branch
git-iris review --from main --to feature-branch --print
```

### Release Workflow

```bash
# Generate changelog
git-iris changelog --from v1.0.0 --update

# Generate release notes
git-iris release-notes --from v1.0.0 > RELEASE_NOTES.md

# Create PR description
git-iris pr --from main > pr_description.md
```

## Debug and Troubleshooting

### Enable Debug Logging

```bash
# Basic logging
git-iris gen --log

# Custom log file
git-iris gen --log --log-file my-debug.log

# Color-coded agent debug
git-iris gen --debug
```

`--debug` surfaces every tool call Iris makes, including the newer code-archaeology helpers — `git_blame`, `git_show`, `repo_map`, and `static_analysis` — alongside the long-standing `git_diff`, `file_read`, `code_search`, `workspace`, `project_docs`, and `parallel_analyze` tools.

### Test Configuration

```bash
# Test with print (no commit)
git-iris gen --print

# Test specific provider
git-iris gen --provider openai --print

# Verify API key works
git-iris review --print
```

### Remote Repository Testing

```bash
# Test against remote repo
git-iris gen --repo https://github.com/user/repo --print
```

## Exit Codes

| Code | Meaning                                    |
| ---- | ------------------------------------------ |
| `0`  | Success                                    |
| `1`  | General error                              |
| `2`  | Configuration error                        |
| `3`  | Git error (not in repo, no staged changes) |
| `4`  | API error (authentication, rate limit)     |

## Environment Variables

See [Environment Variables](../configuration/environment.md) for details.

| Variable            | Purpose                  |
| ------------------- | ------------------------ |
| `OPENAI_API_KEY`    | OpenAI authentication    |
| `ANTHROPIC_API_KEY` | Anthropic authentication |
| `GOOGLE_API_KEY`    | Google authentication    |
| `RUST_LOG`          | Logging and debug output |

## Shell Aliases

Recommended aliases for common operations:

```bash
# ~/.bashrc or ~/.zshrc

# Quick commit
alias gic='git-iris gen --auto-commit'

# Print commit message
alias gim='git-iris gen --print'

# Code review
alias gir='git-iris review --print'

# Launch studio
alias gis='git-iris studio'
```
