//! Parallel Analysis Tool
//!
//! Enables Iris to spawn multiple independent subagents that analyze different
//! portions of a codebase concurrently. This prevents context overflow when
//! dealing with large changesets by distributing work across separate context windows.

use anyhow::Result;
use rig::{
    client::{CompletionClient, ProviderClient},
    completion::{Prompt, ToolDefinition},
    providers::{anthropic, openai},
    tool::Tool,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::agents::debug as agent_debug;
use crate::providers::Provider;

/// Default timeout for individual subagent tasks (2 minutes)
const DEFAULT_SUBAGENT_TIMEOUT_SECS: u64 = 120;

/// Arguments for parallel analysis
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ParallelAnalyzeArgs {
    /// List of analysis tasks to run in parallel.
    /// Each task should be a focused prompt describing what to analyze.
    /// Example: `["Analyze security changes in auth/", "Review performance in db/"]`
    pub tasks: Vec<String>,
}

/// Result from a single subagent analysis
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubagentResult {
    /// The original task prompt
    pub task: String,
    /// The analysis result
    pub result: String,
    /// Whether the analysis succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Aggregated results from all parallel analyses
#[derive(Debug, Serialize, Deserialize)]
pub struct ParallelAnalyzeResult {
    /// Results from each subagent
    pub results: Vec<SubagentResult>,
    /// Number of successful analyses
    pub successful: usize,
    /// Number of failed analyses
    pub failed: usize,
    /// Total execution time in milliseconds
    pub execution_time_ms: u64,
}

/// Provider-specific subagent runner
#[derive(Clone)]
enum SubagentRunner {
    OpenAI {
        client: openai::Client,
        model: String,
    },
    Anthropic {
        client: anthropic::Client,
        model: String,
    },
}

impl SubagentRunner {
    fn new(provider: &str, model: &str, api_key: Option<&str>) -> Result<Self> {
        match provider {
            "openai" => {
                let client = Self::resolve_openai_client(api_key)?;
                Ok(Self::OpenAI {
                    client,
                    model: model.to_string(),
                })
            }
            "anthropic" => {
                let client = Self::resolve_anthropic_client(api_key)?;
                Ok(Self::Anthropic {
                    client,
                    model: model.to_string(),
                })
            }
            _ => Err(anyhow::anyhow!(
                "Unsupported provider for parallel analysis: {}",
                provider
            )),
        }
    }

    /// Create OpenAI client from API key or environment variable
    fn resolve_openai_client(api_key: Option<&str>) -> Result<openai::Client> {
        if let Some(key) = api_key.filter(|k| !k.is_empty()) {
            openai::Client::new(key).map_err(|e| anyhow::anyhow!("Failed to create OpenAI client: {}", e))
        } else if let Ok(key) = std::env::var(Provider::OpenAI.api_key_env()) {
            openai::Client::new(&key).map_err(|e| anyhow::anyhow!("Failed to create OpenAI client: {}", e))
        } else {
            Ok(openai::Client::from_env())
        }
    }

    /// Create Anthropic client from API key or environment variable
    fn resolve_anthropic_client(api_key: Option<&str>) -> Result<anthropic::Client> {
        if let Some(key) = api_key.filter(|k| !k.is_empty()) {
            anthropic::Client::new(key).map_err(|e| anyhow::anyhow!("Failed to create Anthropic client: {}", e))
        } else if let Ok(key) = std::env::var(Provider::Anthropic.api_key_env()) {
            anthropic::Client::new(&key).map_err(|e| anyhow::anyhow!("Failed to create Anthropic client: {}", e))
        } else {
            Ok(anthropic::Client::from_env())
        }
    }

    async fn run_task(&self, task: &str) -> SubagentResult {
        let preamble = "You are a specialized analysis sub-agent. Complete the assigned \
            task thoroughly and return a focused summary.\n\n\
            Guidelines:\n\
            - Use the available tools to gather necessary information\n\
            - Focus only on what's asked\n\
            - Return a clear, structured summary\n\
            - Be concise but comprehensive";

        // Use shared tool registry for consistent tool attachment
        let result = match self {
            Self::OpenAI { client, model } => {
                let builder = client.agent(model).preamble(preamble).max_tokens(4096);
                let agent = crate::attach_core_tools!(builder).build();
                agent.prompt(task).await
            }
            Self::Anthropic { client, model } => {
                let builder = client.agent(model).preamble(preamble).max_tokens(4096);
                let agent = crate::attach_core_tools!(builder).build();
                agent.prompt(task).await
            }
        };

        match result {
            Ok(response) => SubagentResult {
                task: task.to_string(),
                result: response,
                success: true,
                error: None,
            },
            Err(e) => SubagentResult {
                task: task.to_string(),
                result: String::new(),
                success: false,
                error: Some(e.to_string()),
            },
        }
    }
}

/// Parallel analysis tool
/// Spawns multiple subagents to analyze different aspects concurrently
pub struct ParallelAnalyze {
    runner: SubagentRunner,
    model: String,
    /// Timeout in seconds for each subagent task
    timeout_secs: u64,
}

impl ParallelAnalyze {
    /// Create a new parallel analyzer with default timeout
    pub fn new(provider: &str, model: &str, api_key: Option<&str>) -> Result<Self> {
        Self::with_timeout(provider, model, DEFAULT_SUBAGENT_TIMEOUT_SECS, api_key)
    }

    /// Create a new parallel analyzer with custom timeout
    pub fn with_timeout(
        provider: &str,
        model: &str,
        timeout_secs: u64,
        api_key: Option<&str>,
    ) -> Result<Self> {
        // Try the requested provider first, then fall back to openai
        let runner = SubagentRunner::new(provider, model, api_key).or_else(|e| {
            tracing::warn!(
                "Failed to create {} runner: {}, falling back to openai",
                provider,
                e
            );
            SubagentRunner::new("openai", "gpt-4o", api_key)
        })?;

        Ok(Self {
            runner,
            model: model.to_string(),
            timeout_secs,
        })
    }
}

// Use standard tool error macro for consistency
crate::define_tool_error!(ParallelAnalyzeError);

impl Tool for ParallelAnalyze {
    const NAME: &'static str = "parallel_analyze";
    type Error = ParallelAnalyzeError;
    type Args = ParallelAnalyzeArgs;
    type Output = ParallelAnalyzeResult;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Run multiple analysis tasks in parallel using independent subagents. \
                         Each subagent has its own context window, preventing overflow when \
                         analyzing large changesets. Use this when you have multiple independent \
                         analysis tasks that can run concurrently.\n\n\
                         Best for:\n\
                         - Analyzing different directories/modules separately\n\
                         - Processing many commits in batches\n\
                         - Running different types of analysis (security, performance, style) in parallel\n\n\
                         Each task should be a focused prompt. Results are aggregated and returned."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tasks": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of analysis task prompts to run in parallel. Each task runs in its own subagent with independent context.",
                        "minItems": 1,
                        "maxItems": 10
                    }
                },
                "required": ["tasks"]
            }),
        }
    }

    #[allow(clippy::cognitive_complexity)]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        use std::time::Instant;

        let start = Instant::now();
        let tasks = args.tasks;
        let num_tasks = tasks.len();

        agent_debug::debug_context_management(
            "ParallelAnalyze",
            &format!(
                "Spawning {} subagents (fast model: {})",
                num_tasks, self.model
            ),
        );

        // Pre-allocate results vector to preserve task ordering
        let results: Arc<Mutex<Vec<Option<SubagentResult>>>> =
            Arc::new(Mutex::new(vec![None; num_tasks]));

        // Spawn all tasks as parallel tokio tasks, tracking index for ordering
        let mut handles = Vec::new();
        let timeout = Duration::from_secs(self.timeout_secs);
        for (index, task) in tasks.into_iter().enumerate() {
            let runner = self.runner.clone();
            let results = Arc::clone(&results);
            let task_timeout = timeout;
            let timeout_secs = self.timeout_secs;

            let handle = tokio::spawn(async move {
                // Wrap task execution in timeout to prevent hanging
                let result = match tokio::time::timeout(task_timeout, runner.run_task(&task)).await
                {
                    Ok(result) => result,
                    Err(_) => SubagentResult {
                        task: task.clone(),
                        result: String::new(),
                        success: false,
                        error: Some(format!("Task timed out after {} seconds", timeout_secs)),
                    },
                };

                // Store result at original index to preserve ordering
                let mut guard = results.lock().await;
                guard[index] = Some(result);
            });

            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            if let Err(e) = handle.await {
                agent_debug::debug_warning(&format!("Subagent task panicked: {}", e));
            }
        }

        #[allow(clippy::cast_possible_truncation, clippy::as_conversions)]
        let execution_time_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;

        // Extract results, preserving original task order
        let final_results: Vec<SubagentResult> = Arc::try_unwrap(results)
            .map_err(|_| ParallelAnalyzeError("Failed to unwrap results".to_string()))?
            .into_inner()
            .into_iter()
            .enumerate()
            .map(|(i, opt)| {
                opt.unwrap_or_else(|| SubagentResult {
                    task: format!("Task {}", i),
                    result: String::new(),
                    success: false,
                    error: Some("Task did not complete".to_string()),
                })
            })
            .collect();

        let successful = final_results.iter().filter(|r| r.success).count();
        let failed = final_results.iter().filter(|r| !r.success).count();

        agent_debug::debug_context_management(
            "ParallelAnalyze",
            &format!(
                "{}/{} successful in {}ms",
                successful, num_tasks, execution_time_ms
            ),
        );

        Ok(ParallelAnalyzeResult {
            results: final_results,
            successful,
            failed,
            execution_time_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_analyze_args_schema() {
        let schema = schemars::schema_for!(ParallelAnalyzeArgs);
        let json = serde_json::to_string_pretty(&schema).expect("schema should serialize");
        assert!(json.contains("tasks"));
    }
}
