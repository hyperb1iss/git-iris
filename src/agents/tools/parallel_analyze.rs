//! Parallel Analysis Tool
//!
//! Enables Iris to spawn multiple independent subagents that analyze different
//! portions of a codebase concurrently. This prevents context overflow when
//! dealing with large changesets by distributing work across separate context windows.

use anyhow::Result;
use rig::tool::portable::PortableTool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::agents::debug as agent_debug;
use crate::agents::provider::{
    self, CompletionProfile, DynAgent, apply_completion_params, provider_from_name,
};

/// Default timeout for individual subagent tasks (2 minutes)
const DEFAULT_SUBAGENT_TIMEOUT_SECS: u64 = 120;
const DEFAULT_SUBAGENT_MAX_TURNS: usize = 20;

/// Arguments for parallel analysis
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ParallelAnalyzeArgs {
    /// List of analysis tasks to run in parallel.
    /// Each task should be a focused prompt describing what to analyze.
    /// Example: `["Analyze security changes in auth/", "Review performance in db/"]`
    pub tasks: Vec<String>,
    /// Optional turn budget for each subagent. Defaults to the configured value.
    #[serde(default)]
    pub max_turns: Option<usize>,
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

/// A reusable agent configured for focused parallel analysis.
#[derive(Clone)]
struct SubagentRunner {
    agent: DynAgent,
    parent_context: String,
}

impl SubagentRunner {
    async fn run_task(&self, task: &str, max_turns: usize) -> SubagentResult {
        let prompt = format!("{}\n\nDelegated task:\n{}", self.parent_context, task);
        match self.agent.prompt_multi_turn(&prompt, max_turns).await {
            Ok(result) => SubagentResult {
                task: task.to_string(),
                result,
                success: true,
                error: None,
            },
            Err(error) => SubagentResult {
                task: task.to_string(),
                result: format!("Subagent failed: {error}"),
                success: false,
                error: Some(error.to_string()),
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
    /// Default turn budget for each subagent task
    max_turns: usize,
}

impl ParallelAnalyze {
    /// Attach the parent task scope and constraints to every delegated task.
    #[must_use]
    pub fn with_parent_context(mut self, context: String) -> Self {
        self.runner.parent_context = context;
        self
    }

    /// Create a new parallel analyzer with default timeout
    ///
    /// # Errors
    ///
    /// Returns an error when the requested provider runner cannot be created.
    pub fn new(provider: &str, model: &str, api_key: Option<&str>) -> Result<Self> {
        Self::with_timeout(
            provider,
            model,
            DEFAULT_SUBAGENT_TIMEOUT_SECS,
            api_key,
            None,
        )
    }

    /// Create a new parallel analyzer with custom timeout
    ///
    /// # Errors
    ///
    /// Returns an error when the requested provider runner cannot be created.
    pub fn with_timeout(
        provider: &str,
        model: &str,
        timeout_secs: u64,
        api_key: Option<&str>,
        additional_params: Option<HashMap<String, String>>,
    ) -> Result<Self> {
        Self::with_limits(
            provider,
            model,
            timeout_secs,
            DEFAULT_SUBAGENT_MAX_TURNS,
            api_key,
            additional_params,
        )
    }

    /// Create a new parallel analyzer with custom timeout and turn budget.
    /// The turn budget is clamped to 1..=100.
    ///
    /// # Errors
    ///
    /// Returns an error when the requested provider runner cannot be created.
    pub fn with_limits(
        provider: &str,
        model: &str,
        timeout_secs: u64,
        max_turns: usize,
        api_key: Option<&str>,
        additional_params: Option<HashMap<String, String>>,
    ) -> Result<Self> {
        let additional_params = additional_params.unwrap_or_default();
        let provider_name = provider_from_name(provider)?;
        let builder = provider::agent_builder(provider_name, model, api_key)?;
        let builder = apply_completion_params(
            builder,
            provider_name,
            model,
            4096,
            Some(&additional_params),
            CompletionProfile::Subagent,
        );
        Ok(Self::from_builder(builder, model, timeout_secs, max_turns))
    }

    pub(crate) fn from_builder(
        builder: rig::agent::AgentBuilder,
        model: &str,
        timeout_secs: u64,
        max_turns: usize,
    ) -> Self {
        let builder = builder.preamble(crate::agents::prompts::SUBAGENT_PREAMBLE);
        let agent = DynAgent(crate::attach_core_tools!(builder).build());
        Self {
            runner: SubagentRunner {
                agent,
                parent_context: String::new(),
            },
            model: model.to_string(),
            timeout_secs,
            max_turns: max_turns.clamp(1, 100),
        }
    }
}

// Use standard tool error macro for consistency
crate::define_tool_error!(ParallelAnalyzeError);

impl PortableTool for ParallelAnalyze {
    const NAME: &'static str = "parallel_analyze";
    type Error = ParallelAnalyzeError;
    type Args = ParallelAnalyzeArgs;
    type Output = ParallelAnalyzeResult;

    fn description(&self) -> String {
        "Run multiple analysis tasks in parallel using independent subagents. \
                         Each subagent has its own context window, preventing overflow when \
                         analyzing large changesets. Use this when you have multiple independent \
                         analysis tasks that can run concurrently.\n\n\
                         Best for:\n\
                         - Analyzing different directories/modules separately\n\
                         - Processing many commits in batches\n\
                         - Running different types of analysis (security, performance, style) in parallel\n\n\
                         Each task should be a focused prompt. Results are aggregated and returned."
                .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "tasks": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of analysis task prompts to run in parallel. Each task runs in its own subagent with independent context.",
                    "minItems": 1,
                    "maxItems": 10
                },
                "max_turns": {
                    "type": "integer",
                    "description": "Optional per-subagent turn budget. Increase for broad repository searches; lower it to cap cost or runaway tool loops.",
                    "minimum": 1,
                    "maximum": 100
                }
            },
            "required": ["tasks"]
        })
    }

    #[allow(clippy::cognitive_complexity)]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        use std::time::Instant;

        let start = Instant::now();
        let max_turns = args.max_turns.unwrap_or(self.max_turns).clamp(1, 100);
        let tasks = args.tasks;
        let num_tasks = tasks.len();

        agent_debug::debug_context_management(
            "ParallelAnalyze",
            &format!(
                "Spawning {} subagents (fast model: {}, max turns: {})",
                num_tasks, self.model, max_turns
            ),
        );

        let initial_results = tasks
            .iter()
            .map(|task| {
                Some(SubagentResult {
                    task: task.clone(),
                    result: "Subagent task did not complete".to_string(),
                    success: false,
                    error: Some("Task did not complete".to_string()),
                })
            })
            .collect();
        let results: Arc<Mutex<Vec<Option<SubagentResult>>>> =
            Arc::new(Mutex::new(initial_results));

        // Spawn all tasks as parallel tokio tasks, tracking index for ordering
        let mut handles = Vec::new();
        let repo_root = super::common::current_repo_root()?;
        let trusted = super::common::current_repo_execution_trusted();
        let timeout = Duration::from_secs(self.timeout_secs);
        for (index, task) in tasks.into_iter().enumerate() {
            let runner = self.runner.clone();
            let results = Arc::clone(&results);
            let task_timeout = timeout;
            let timeout_secs = self.timeout_secs;
            let task_max_turns = max_turns;

            let task_repo_root = repo_root.clone();
            let handle = tokio::spawn(async move {
                // Wrap task execution in timeout to prevent hanging
                let result = match tokio::time::timeout(
                    task_timeout,
                    super::common::with_repo_execution_context(
                        &task_repo_root,
                        trusted,
                        runner.run_task(&task, task_max_turns),
                    ),
                )
                .await
                {
                    Ok(result) => result,
                    Err(_) => SubagentResult {
                        task: task.clone(),
                        result: format!("Subagent timed out after {timeout_secs} seconds"),
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
                    result: "Subagent task did not complete".to_string(),
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
        assert!(json.contains("max_turns"));
    }
}
