# git-llm Architecture Design

## 1. Components

### 1.1 CLI Interface
- Handles user input and output
- Parses command-line arguments

### 1.2 Git Integration
- Retrieves relevant Git information (diff, branch name, etc.)
- Applies the final commit message

### 1.3 LLM Client
- Manages communication with the LLM API
- Handles API authentication and rate limiting

### 1.4 Prompt Manager
- Stores and manages system prompts
- Constructs the full prompt by combining system prompts and user input

### 1.5 Response Processor
- Processes and formats the LLM's response
- Applies any necessary post-processing or filtering

## 2. Data Flow

1. User invokes git-llm with an initial commit message
2. CLI Interface parses the input
3. Git Integration retrieves relevant Git information
4. Prompt Manager constructs the full prompt
5. LLM Client sends the prompt to the LLM API
6. Response Processor handles the LLM's response
7. CLI Interface presents the refined commit message to the user
8. Git Integration applies the commit (if auto-commit is enabled)

## 3. System Prompts Strategy

We'll use a tiered approach for system prompts:

1. Base Prompt: Provides general instructions for commit message refinement
2. Context-Specific Prompts: Tailored to specific scenarios (e.g., feature additions, bug fixes)
3. User-Defined Prompts: Allows users to add custom instructions

Prompts will be stored as YAML files for easy management and modification.

## 4. Git Data Strategy

To provide context to the LLM, we'll include:

1. Git diff of staged changes
2. Current branch name
3. Recent commit history (last 5 commits)
4. Project-specific information (e.g., from .gitattributes or a custom config file)

## 5. Configuration

We'll use a config file (`~/.gitllmconfig` or `.gitllm` in the project root) to store:

1. LLM API credentials
2. Preferred LLM model
3. Custom system prompts
4. User preferences (e.g., auto-commit, diff inclusion)

## 6. Error Handling and Logging

- Implement comprehensive error handling for network issues, API errors, and Git operation failures
- Use a logging system to aid in debugging and provide verbose output when requested

## 7. Testing Strategy

1. Unit tests for individual components
2. Integration tests for the complete flow
3. Mock LLM API for testing without actual API calls

## 8. Future Expansions

1. Support for multiple LLM providers
2. Integration with CI/CD pipelines
3. GUI interface
4. Commit message templates based on project history
