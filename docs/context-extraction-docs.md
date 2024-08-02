# Git-Iris Context Extraction System

## 1. Overview

The Context Extraction System is a core component of Git-Iris, responsible for gathering and processing relevant information from the Git repository to provide rich, meaningful context for AI-generated commit messages. This system plays a crucial role in enhancing the quality and accuracy of the generated messages.

## 2. Purpose

The main purposes of the Context Extraction System are:

1. To collect comprehensive information about the current state of the Git repository
2. To analyze changes in staged files and provide meaningful insights
3. To extract project metadata for better understanding of the codebase
4. To optimize the extracted information for use with Language Models (LLMs)

## 3. Components

### 3.1 Git Information Extractor

This component interfaces with the Git repository to extract essential information:

- Current branch
- Recent commits
- Staged files and their changes
- Unstaged files
- Project root directory

### 3.2 File Analyzers

Language-specific analyzers that process the content and changes in staged files:

- Implement the `FileAnalyzer` trait
- Provide detailed analysis of file changes
- Extract relevant metadata from files

### 3.3 Project Metadata Extractor

Gathers overall project information:

- Programming languages used
- Frameworks and libraries
- Version information
- Build systems and test frameworks

### 3.4 Token Optimizer

Ensures the extracted context fits within the token limits of the LLM:

- Implements intelligent truncation strategies
- Prioritizes the most relevant information

## 4. Context Extraction Process

### 4.1 Git Repository Analysis

1. Determine the current branch
2. Fetch recent commits (default: last 5)
3. Identify staged and unstaged files
4. Extract diffs for staged files

### 4.2 File Analysis

For each staged file:

1. Determine the file type
2. Apply the appropriate `FileAnalyzer`
3. Extract relevant changes and metadata
4. Generate a summary of modifications

### 4.3 Project Metadata Extraction

1. Scan the project directory for relevant files (e.g., package.json, Cargo.toml)
2. Extract project-wide information
3. Identify primary programming languages and frameworks

### 4.4 Context Optimization

1. Combine all extracted information
2. Apply token optimization strategies
3. Ensure the context fits within the specified token limit

## 5. Data Structures

### 5.1 CommitContext

The main structure holding all extracted context:

```rust
pub struct CommitContext {
    pub branch: String,
    pub recent_commits: Vec<RecentCommit>,
    pub staged_files: Vec<StagedFile>,
    pub unstaged_files: Vec<String>,
    pub project_metadata: ProjectMetadata,
}
```

### 5.2 StagedFile

Represents a staged file and its analysis:

```rust
pub struct StagedFile {
    pub path: String,
    pub change_type: ChangeType,
    pub diff: String,
    pub analysis: Vec<String>,
    pub content_excluded: bool,
}
```

### 5.3 ProjectMetadata

Holds project-wide information:

```rust
pub struct ProjectMetadata {
    pub language: Option<String>,
    pub framework: Option<String>,
    pub dependencies: Vec<String>,
    pub version: Option<String>,
    pub build_system: Option<String>,
    pub test_framework: Option<String>,
    pub plugins: Vec<String>,
}
```

## 6. Key Functions

### 6.1 get_git_info

```rust
pub fn get_git_info(repo_path: &Path, config: &Config) -> Result<CommitContext>
```

This function orchestrates the entire context extraction process:

1. Opens the Git repository
2. Extracts branch, commits, and file statuses
3. Analyzes staged files
4. Gathers project metadata
5. Optimizes the context based on token limits

### 6.2 analyze_staged_file

```rust
fn analyze_staged_file(file: &StagedFile, analyzer: &dyn FileAnalyzer) -> Vec<String>
```

Applies a specific `FileAnalyzer` to a staged file and returns the analysis results.

### 6.3 extract_project_metadata

```rust
fn extract_project_metadata(repo_path: &Path) -> Result<ProjectMetadata>
```

Scans the repository for project-specific files and extracts relevant metadata.

## 7. Token Optimization Strategies

The Context Extraction System employs several strategies to optimize token usage:

1. Truncation of long diffs while preserving the most relevant parts
2. Summarization of repeated patterns in changes
3. Prioritization of recent commits and more significant changes
4. Exclusion of binary files or large generated files

## 8. Extensibility

The Context Extraction System is designed for easy extensibility:

1. New `FileAnalyzer` implementations can be added for additional file types
2. The project metadata extraction can be extended to support new build systems or frameworks
3. Additional optimization strategies can be implemented in the `TokenOptimizer`

## 9. Performance Considerations

To ensure optimal performance:

1. File analysis is performed only on staged files
2. Large repositories use pagination for fetching commits and file statuses
3. Caching mechanisms can be implemented for frequently analyzed files or projects

## 10. Future Improvements

Potential areas for enhancement include:

1. Integration with code quality tools for additional context
2. Support for analyzing commit history patterns
3. Machine learning-based relevance scoring for extracted information
4. Parallel processing of file analysis for large changesets

## 11. Example Usage

Here's a simplified example of how the Context Extraction System is used in Git-Iris:

```rust
let config = Config::load()?;
let repo_path = std::env::current_dir()?;
let context = get_git_info(&repo_path, &config)?;

// Use the extracted context to generate a commit message
let commit_message = generate_commit_message(&context, &config)?;
```

