# Git-Iris File Analyzer System

## 1. Overview

The File Analyzer System is a core component of Git-Iris, responsible for extracting meaningful information from changed files in a Git repository. This system plays a crucial role in providing context for AI-generated commit messages.

## 2. Purpose

The main purposes of the File Analyzer System are:

1. To analyze the content and structure of changed files
2. To extract relevant information that can inform the commit message generation process
3. To provide language-specific insights for improved context

## 3. Components

### 3.1 FileAnalyzer Trait

The `FileAnalyzer` trait defines the interface for all file analyzers:

```rust
pub trait FileAnalyzer {
    fn analyze(&self, file: &str, staged_file: &StagedFile) -> Vec<String>;
    fn get_file_type(&self) -> &'static str;
    fn extract_metadata(&self, file: &str, content: &str) -> ProjectMetadata;
}
```

### 3.2 Language-Specific Analyzers

Git-Iris implements several language-specific analyzers, including:

- Rust Analyzer
- JavaScript/TypeScript Analyzer
- Python Analyzer
- Java Analyzer
- C/C++ Analyzer
- Go Analyzer
- YAML Analyzer
- JSON Analyzer
- Markdown Analyzer

Each analyzer implements the `FileAnalyzer` trait with language-specific logic.

### 3.3 Default Analyzer

A default analyzer is provided for unsupported file types, ensuring that all files can be processed even if detailed analysis is not available.

### 3.4 Analyzer Factory

The `get_analyzer` function serves as a factory, returning the appropriate analyzer based on the file extension:

```rust
pub fn get_analyzer(file: &str) -> Box<dyn FileAnalyzer> {
    // Logic to select and return the appropriate analyzer
}
```

## 4. Functionality

### 4.1 File Analysis

Each analyzer implements the `analyze` method, which:

1. Examines the file's diff
2. Identifies important changes (e.g., modified functions, classes, or structures)
3. Extracts relevant information
4. Returns a vector of strings describing the changes

### 4.2 Metadata Extraction

The `extract_metadata` method is responsible for:

1. Identifying the programming language
2. Detecting frameworks or libraries in use
3. Extracting version information (if available)
4. Identifying build systems or test frameworks

### 4.3 File Type Identification

The `get_file_type` method returns a human-readable description of the file type, useful for generating more informative commit messages.

## 5. Integration with Git-Iris

The File Analyzer System integrates with other components of Git-Iris:

1. The Git Integration component provides diffs and file information to the analyzers.
2. Analysis results are used by the Prompt Management component to construct informative prompts for LLMs.
3. The Token Optimization component may prioritize or truncate analyzer output based on relevance and token limits.

## 6. Extensibility

The File Analyzer System is designed for easy extensibility:

1. New language analyzers can be added by implementing the `FileAnalyzer` trait.
2. The analyzer factory can be extended to support new file types.
3. Existing analyzers can be enhanced to provide more detailed analysis.

## 7. Performance Considerations

To ensure optimal performance:

1. Analyzers focus on extracting high-value information efficiently.
2. Large files or diffs may be partially analyzed to stay within reasonable time and memory constraints.
3. Caching mechanisms can be implemented for frequently analyzed files or projects.

## 8. Future Improvements

Potential areas for enhancement include:

1. Machine learning-based analysis for more accurate change detection
2. Support for more programming languages and file types
3. Integration with abstract syntax tree (AST) parsers for deeper code understanding
4. Collaborative filtering to improve analysis based on user feedback

## 9. Example: Rust Analyzer

Here's a simplified example of how the Rust Analyzer might be implemented:

```rust
pub struct RustAnalyzer;

impl FileAnalyzer for RustAnalyzer {
    fn analyze(&self, _file: &str, staged_file: &StagedFile) -> Vec<String> {
        let mut analysis = Vec::new();

        if let Some(functions) = extract_modified_functions(&staged_file.diff) {
            analysis.push(format!("Modified functions: {}", functions.join(", ")));
        }

        if let Some(structs) = extract_modified_structs(&staged_file.diff) {
            analysis.push(format!("Modified structs: {}", structs.join(", ")));
        }

        // Additional Rust-specific analysis...

        analysis
    }

    fn get_file_type(&self) -> &'static str {
        "Rust source file"
    }

    fn extract_metadata(&self, _file: &str, content: &str) -> ProjectMetadata {
        let mut metadata = ProjectMetadata::default();
        metadata.language = Some("Rust".to_string());

        // Extract Rust-specific metadata...

        metadata
    }
}
```

This example demonstrates how language-specific analysis can be implemented to provide valuable context for commit message generation.

The File Analyzer System is a critical component in Git-Iris's ability to generate meaningful, context-aware commit messages across various programming languages and file types.