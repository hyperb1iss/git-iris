━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.0] - 

### 🗑️ Removed

- 🔥 Remove tracking of unstaged files across multiple modules (db9db44)
- 🔥 Delete legacy interactive and old TUI commit modules (630aa21)

### ✨ Added

- ✨ Introduce cosmic-themed TUI for commit message creation (99c9428)
- ✨ Add support for pre and post commit hooks (43c8b56)
- ✨ Implement retry mechanism for LLM requests with exponential backoff (b798758)
- 🚀 Integrate Gitmoji support in TUI for commit messages (217ed78)
- 📝 Create TODO.md file with project roadmap and goals (3e18ffa)
- 🎨 Enhance instruction presets with emojis for visual appeal (7927873)

### 🐛 Fixed

- 🐛 Fix TUI message editing and rendering issues (538552f)
- 🐛 Correct binary file detection in git status parsing (a95c228)
- 🐛 Address CI/CD release issues and improve asset handling (da7b239)

### 🔄 Changed

- ♻️ Refactor project structure for improved modularity and maintainability (f1d60bf, e67206d, b48d37a)
- ⚡️ Optimize performance by parallelizing metadata extraction and caching git context (3a8163d, f1d60bf)
- 🔧 Update logging configuration for flexible log file paths and optional stdout logging (d738d89)
- 📝 Revise README to reflect new Git workflow focus and update project description (c404eb5)

### 📊 Metrics

- Total Commits: 70
- Files Changed: 257
- Insertions: 9691
- Deletions: 6079

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
