# Git-LLM Context Extraction Process

## Overview

The git-llm tool extracts comprehensive context from the current Git repository to provide rich, relevant information to the Language Model (LLM) for generating accurate commit messages. This document outlines the context extraction process.

## Context Components

The context extraction process gathers the following components:

1. Current Branch
2. Recent Commits
3. Staged Files and Their Changes
4. Unstaged Files
5. Project Root Directory

## Extraction Process

### 1. Current Branch

- **Function**: `get_current_branch()`
- **Command Used**: `git rev-parse --abbrev-ref HEAD`
- **Purpose**: Identifies the branch on which the commit will be made, providing important context for the commit message.

### 2. Recent Commits

- **Function**: `get_recent_commits(count: usize)`
- **Command Used**: `git log -[count] --oneline`
- **Purpose**: Retrieves the last `count` commits (default is 5) to provide historical context for the current changes.

### 3. Staged Files and Their Changes

- **Function**: `get_staged_files_with_diff()`
- **Commands Used**:
  - `git status --porcelain`: To identify staged files and their status
  - `git diff --cached [file]`: To get the diff for each staged file
- **Process**:
  1. Parse the output of `git status --porcelain` to identify staged files.
  2. For each staged file, determine its status:
     - 'A': Added
     - 'M': Modified
     - 'D': Deleted
  3. Fetch the diff for each staged file.
  4. Store the file path, status, and diff in a `HashMap`.
- **Purpose**: Provides detailed information about what changes are being committed, which is crucial for generating an accurate commit message.

### 4. Unstaged Files

- **Function**: `get_unstaged_files()`
- **Command Used**: `git ls-files --others --exclude-standard`
- **Purpose**: Lists files that are in the working directory but not staged. This provides context about the state of the repository beyond the current commit.

### 5. Project Root Directory

- **Function**: `get_project_root()`
- **Command Used**: `git rev-parse --show-toplevel`
- **Purpose**: Identifies the root directory of the Git repository, providing context about the project structure.

## Data Structure

All extracted information is stored in a `GitInfo` struct:

```rust
pub struct GitInfo {
    pub branch: String,
    pub recent_commits: Vec<String>,
    pub staged_files: HashMap<String, FileChange>,
    pub unstaged_files: Vec<String>,
    pub project_root: String,
}

pub struct FileChange {
    pub status: String,
    pub diff: String,
}
```

## Prompt Creation

The extracted context is used to create a comprehensive prompt for the LLM:

1. The project root and current branch are included for high-level context.
2. Recent commits are listed to provide historical context.
3. Staged files are detailed with their status and a truncated diff (limited to 500 characters per file to manage prompt length).
4. Unstaged files are listed to give a complete picture of the repository state.

## Considerations

- **Performance**: The context extraction process involves multiple Git commands, which may impact performance for very large repositories or when there are many changed files.
- **Privacy**: Care should be taken when using this tool in repositories with sensitive information, as file contents are sent to the LLM as part of the diff.
- **LLM Token Limits**: The amount of context extracted may need to be adjusted based on the token limits of the LLM being used.

## Future Improvements

- Implement caching mechanisms to improve performance for repeated runs.
- Add options to customize the amount of context extracted (e.g., number of recent commits, diff truncation length).
- Implement filters to exclude certain files or directories from the context.
