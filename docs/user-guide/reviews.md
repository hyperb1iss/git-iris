# Code Reviews

Iris performs structured code analysis, returning severity-graded findings with location, category, confidence, and (optionally) suggested fixes — across concerns like security, performance, error handling, complexity, testing, and more.

## Quick Example

```bash
# Review staged changes
git-iris review

# Review specific commit
git-iris review --commit abc1234

# Review branch comparison
git-iris review --from main --to feature-branch

# Include unstaged changes
git-iris review --include-unstaged
```

## Command Reference

```bash
git-iris review [FLAGS] [OPTIONS]
```

### Key Flags

| Flag                            | Description                                               |
| ------------------------------- | --------------------------------------------------------- |
| `-p, --print`                   | Print review to stdout and exit                           |
| `--raw`                         | Output raw markdown without console formatting            |
| `--include-unstaged`            | Include unstaged changes in review                        |
| `--commit <ref>`                | Review specific commit (hash, branch, or reference)       |
| `--from <ref>`                  | Starting reference for comparison (defaults to `main`)    |
| `--to <ref>`                    | Target reference for comparison                           |
| `--github-review`               | Publish the review as a GitHub PR review comment          |
| `--pr <number>`                 | GitHub PR number for publishing                           |
| `--github-inline-comments`      | Add validated inline comments for findings in the PR diff |
| `--github-review-event <event>` | Review event: `comment`, `request-changes`, or `approve`  |

### Global Options

| Option                      | Description                                                             |
| --------------------------- | ----------------------------------------------------------------------- |
| `--provider <name>`         | Override LLM provider                                                   |
| `--model <name>`            | Override model for this operation                                       |
| `-r, --repo <url>`          | Run against a remote repository URL instead of the local repo           |
| `--preset <name>`           | Use instruction preset                                                  |
| `-i, --instructions "text"` | Custom review focus                                                     |
| `--critic` / `--no-critic`  | Run or skip the critic verification pass after generation (default: on) |
| `--debug`                   | Show agent execution details                                            |

## Review Modes

### Staged Changes (Default)

Review what's currently staged:

```bash
git-iris review
```

Analyzes all staged changes as a cohesive unit.

### Include Unstaged

Review both staged and unstaged changes:

```bash
git-iris review --include-unstaged
```

Useful for pre-commit analysis of all working changes.

### Specific Commit

Review a single commit:

```bash
# By hash
git-iris review --commit abc1234

# By branch name
git-iris review --commit feature-branch

# By reference
git-iris review --commit HEAD~1
```

### Branch Comparison

Review differences between branches:

```bash
# Compare feature branch to main
git-iris review --from main --to feature-branch

# From main to current branch (auto-detects HEAD)
git-iris review --to feature-branch

# Custom base branch
git-iris review --from develop --to feature-xyz
```

## Finding Categories

Each finding Iris produces is tagged with one of these categories (the `Category` enum in source):

| Category           | Focus                                                        |
| ------------------ | ------------------------------------------------------------ |
| **security**       | Vulnerabilities, unsafe patterns, missing input validation   |
| **performance**    | Inefficient algorithms, resource leaks, hot-path regressions |
| **error_handling** | Edge cases, error propagation, recovery gaps                 |
| **complexity**     | Deep nesting, god functions, hard-to-reason logic            |
| **abstraction**    | Leaky abstractions, unclear separation of concerns           |
| **duplication**    | Copy-pasted code, repeated logic                             |
| **testing**        | Gaps in coverage, brittle or missing tests                   |
| **style**          | Inconsistencies, naming, formatting                          |
| **api_contract**   | Breaking changes, public surface drift                       |
| **concurrency**    | Race conditions, locking errors, async correctness           |
| **documentation**  | Missing or misleading docs and comments                      |
| **other**          | Anything that doesn't fit cleanly above                      |

## Output Format

Reviews are emitted as structured findings rendered into markdown. The shape is:

```markdown
# Code Review

## Summary

High-level overview of the changeset and overall assessment.

## Review Coverage

Risk: medium

Strategy: Plan → run targeted specialist passes → reconcile findings.

Specialist passes:

- Security pass on auth changes
- Concurrency pass on the session manager

## Findings

Reviewed 7 file(s). Found 3 issue(s): 1 critical, 1 high, 1 medium, 0 low.

### CRITICAL

- [CRITICAL] **Hardcoded secret in config loader in `src/config/loader.rs:42`**
  Category: security. Confidence: 92%.
  The default config path embeds an HMAC key that ships with the binary.
  **Fix**: Read the key from the environment and fail loudly if absent.
  Evidence: src/config/loader.rs:42, src/config/loader.rs:48

### HIGH

- [HIGH] **Unbounded retry loop on auth failure in `src/auth/refresh.rs:118`**
  Category: error_handling. Confidence: 81%.
  ...

### MEDIUM

- [MEDIUM] **Duplicate token-validation logic in `src/auth/middleware.rs:60`**
  Category: duplication. Confidence: 74%.
  ...
```

Each finding includes a severity tag (`CRITICAL` / `HIGH` / `MEDIUM` / `LOW`), a title with file and line, a category, a confidence percentage, a body, and optional `**Fix**:` and `Evidence:` lines. Findings are grouped by severity; there are no per-dimension sections.

## Confidence Gating

Iris filters findings to those with at least **70% confidence** before rendering or publishing. The model may produce lower-confidence observations during reasoning, but only the high-confidence ones reach the output (and the summary counts reflect the filtered set, not the raw count). If you're seeing fewer findings than expected, the others fell below the gate.

## Customizing Reviews

### Using Presets

```bash
# Concise review focusing on critical issues
git-iris review --preset concise

# Detailed analysis with explanations
git-iris review --preset detailed

# Technical deep dive
git-iris review --preset technical
```

### Custom Instructions

```bash
# Security-focused review
git-iris review --instructions "Focus on security vulnerabilities and authentication"

# Performance review
git-iris review --instructions "Analyze performance impacts and database queries"

# Architecture review
git-iris review --instructions "Evaluate design patterns and code organization"
```

## Output Modes

### Interactive (Default)

Pretty-printed to console with syntax highlighting:

```bash
git-iris review
```

### Print Mode

Clean output for piping:

```bash
# Save to file
git-iris review --print > review.md

# Pipe to pager
git-iris review --print | less
```

### Raw Mode

Pure markdown without ANSI formatting:

```bash
# For CI/CD pipelines
git-iris review --raw > review.md

# For markdown processors
git-iris review --raw | pandoc -f markdown -t html
```

### GitHub Review Publishing

Publish a review of an open GitHub PR:

```bash
# Auto-detect the PR from the current branch
git-iris review --github-review

# Or target a specific PR
git-iris review --from main --to feature-branch --github-review --pr 123

# Request changes when publishing
git-iris review --github-review --github-review-event request-changes

# Add an inline comment per finding (uses each finding's file + start/end line)
git-iris review --github-review --github-inline-comments
```

Publishing resolves the PR before analysis and pins its base and head commits. Without an explicit
commit or range, Iris reviews the PR from its merge base to its head. The commits must be available
locally. An explicit `--commit` or `--to` must match the PR head; unpublished working-tree changes
cannot be attached to a GitHub commit review. If the PR head changes or the PR targets a different
base branch during analysis, publication stops and asks for a new review. Normal base-branch
advancement does not discard the analysis. A push after the final check cannot relabel the review onto newer code:
the submitted review retains the analyzed commit ID.

When `--github-inline-comments` is set, findings at or above the 70% confidence gate are matched to
reviewable lines in the PR diff. Findings outside those lines remain in the review body.

The review body is also augmented with a `## GitHub Permalinks` section listing one permalink per visible finding back to the exact commit and lines, so reviewers can jump straight to the code.

PR auto-detection looks at the current branch. It fails clearly in two cases:

- **Detached HEAD** — no branch to infer a PR from. Pass `--pr <number>`.
- **Zero or multiple open PRs for the branch** — Iris can't pick for you. Pass `--pr <number>`.

Git-Iris reads `GH_TOKEN` / `GITHUB_TOKEN`, then falls back to the GitHub CLI auth store.

## Integration Workflows

### Pre-Commit Hook

Add to `.git/hooks/pre-commit`:

```bash
#!/bin/bash
git-iris review --print --quiet
```

### CI/CD Pipeline

```yaml
# GitHub Actions example
- name: AI Code Review
  run: |
    git-iris review --from ${{ github.base_ref }} --to ${{ github.head_ref }} --raw > review.md
    gh pr comment --body-file review.md
```

### Git Alias

Add to `~/.gitconfig`:

```ini
[alias]
    ai-review = !git-iris review
    review-commit = !git-iris review --commit
```

Usage:

```bash
git ai-review
git review-commit abc1234
```

## How Iris Reviews Code

Beyond reading the diff, Iris uses a handful of investigative tools to ground her findings:

- **`static_analysis`** — runs the project's configured linters (e.g. `cargo clippy`, `eslint`, `ruff`) on changed files. If a linter already flags an issue, Iris won't duplicate it as a speculative manual finding; if it reports failures on changed code, those get prioritized.
- **`repo_map`** — ranked overview of the codebase, used to orient before drilling in.
- **`git_blame`** — line-level history to understand who touched what and when.
- **`git_show`** — inspects historical commits when context older than the diff matters.

This is why review findings tend to be specific and citation-backed rather than generic style nitpicks.

## Tips

**For Large Changes:**

- Iris uses parallel subagent analysis for 20+ files
- Break large reviews into smaller chunks when possible
- Use `--from` and `--to` to review specific ranges

**For Security Focus:**

```bash
git-iris review --instructions "Deep security audit focusing on authentication, authorization, and data validation"
```

**For Performance Analysis:**

```bash
git-iris review --preset technical --instructions "Focus on performance bottlenecks and optimization opportunities"
```

**For Quick Checks:**

```bash
git-iris review --preset concise --print
```

## Examples

```bash
# Review staged changes with detailed analysis
git-iris review --preset detailed

# Security-focused review of branch
git-iris review --from main --to security-fixes --instructions "Focus on security"

# Quick review of last commit
git-iris review --commit HEAD~1 --preset concise --print

# Review PR changes
git-iris review --from origin/main --to feature-branch --raw

# Include unstaged for complete analysis
git-iris review --include-unstaged --preset detailed

# Debug agent execution
git-iris review --debug
```

## Error Handling

**No Changes to Review:**

```
⚠ No changes found to review
→ Stage changes with 'git add' or specify a commit/range
```

**Invalid Reference:**

```
✗ Invalid Git reference: 'nonexistent-branch'
→ Use 'git log' to find valid commits/branches
```

**Conflicting Options:**

```
✗ Cannot use --commit with --from/--to
→ Use either --commit for single commit or --from/--to for ranges
```
