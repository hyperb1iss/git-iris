//! Agent task spawning for Iris Studio
//!
//! Contains all async task spawning functions for Iris agent operations.

use crate::types::GeneratedMessage;

use super::{ChatUpdateType, IrisTaskResult, StudioApp};
use crate::studio::events::{BlameInfo, SemanticBlameResult, TaskType};

impl StudioApp {
    // ═══════════════════════════════════════════════════════════════════════════════
    // Generic Structured Task Spawner
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Spawn a structured (non-streaming) agent task.
    ///
    /// Handles the common pattern shared by review, PR, changelog, and release notes:
    /// agent availability check → status messages → `tokio::spawn` → result channel.
    fn spawn_structured_task<F, Fut>(
        &self,
        task_type: TaskType,
        agent_task: &super::super::events::AgentTask,
        task_fn: F,
    ) where
        F: FnOnce(std::sync::Arc<crate::agents::IrisAgentService>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = IrisTaskResult> + Send,
    {
        let Some(agent) = self.agent_service.clone() else {
            let tx = self.iris_result_tx.clone();
            let _ = tx.send(IrisTaskResult::Error {
                task_type,
                error: "Agent service not available".to_string(),
            });
            return;
        };

        self.spawn_status_messages(agent_task);
        let tx = self.iris_result_tx.clone();

        tokio::spawn(async move {
            let result = task_fn(agent).await;
            let _ = tx.send(result);
        });
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Chat Query
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Spawn a task for chat query - uses Iris agent with chat capability
    pub(super) fn spawn_chat_query(
        &self,
        message: String,
        context: crate::studio::events::ChatContext,
    ) {
        use super::super::events::AgentTask;
        use crate::agents::StructuredResponse;
        use crate::agents::status::IRIS_STATUS;
        use crate::agents::tools::create_content_update_channel;
        use crate::studio::state::{ChatMessage, ChatRole};
        use tokio_util::sync::CancellationToken;

        let Some(agent) = self.agent_service.clone() else {
            let tx = self.iris_result_tx.clone();
            let _ = tx.send(IrisTaskResult::ChatResponse(
                "Agent service not available".to_string(),
            ));
            return;
        };

        // Spawn dynamic status messages
        let task = AgentTask::Chat {
            message: message.clone(),
            context: context.clone(),
        };
        self.spawn_status_messages(&task);

        // Create bounded content update channel for tool-based updates
        let (content_tx, content_rx) = create_content_update_channel();

        // Capture context before spawning async task
        let tx = self.iris_result_tx.clone();
        let tx_status = self.iris_result_tx.clone();
        let tx_updates = self.iris_result_tx.clone();
        let mode = context.mode;

        // Extract conversation history (convert VecDeque → Vec)
        let chat_history: Vec<ChatMessage> =
            self.state.chat_state.messages.iter().cloned().collect();

        // Use context content if provided, otherwise extract from state
        let current_content = self.chat_content_context(context.current_content);

        // Cancellation token to signal when the main task is done
        let cancel_token = CancellationToken::new();
        let cancel_status = cancel_token.clone();
        let cancel_updates = cancel_token.clone();

        // Spawn a status polling task (polls global state, so still uses interval)
        tokio::spawn(async move {
            use crate::agents::status::IrisPhase;
            let mut last_tool: Option<String> = None;
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));

            loop {
                tokio::select! {
                    () = cancel_status.cancelled() => break,
                    _ = interval.tick() => {
                        let status = IRIS_STATUS.get_current();

                        // Check if we're in a tool execution phase
                        if let IrisPhase::ToolExecution {
                            ref tool_name,
                            ref reason,
                        } = status.phase
                        {
                            // Only send if it's a new tool
                            if last_tool.as_ref() != Some(tool_name) {
                                let _ = tx_status.send(IrisTaskResult::ToolStatus {
                                    tool_name: tool_name.clone(),
                                    message: reason.clone(),
                                });
                                last_tool = Some(tool_name.clone());
                            }
                        }
                    }
                }
            }
        });

        // Spawn a task to listen for content updates from tools (uses select! for zero latency)
        tokio::spawn(forward_content_updates(
            content_rx,
            tx_updates,
            cancel_updates,
        ));

        tokio::spawn(async move {
            // Build comprehensive context (universal chat across all modes)
            let mode_context = format!(
                "Current Mode: {:?}\nYou are Iris, a helpful git assistant. You have access to all generated content across modes and can help with commit messages, PR descriptions, code reviews, changelogs, and release notes.",
                mode
            );

            // Build conversation history string
            let history_str = if chat_history.is_empty() {
                String::new()
            } else {
                let mut hist = String::from("\n## Conversation History\n");
                for msg in &chat_history {
                    match msg.role {
                        ChatRole::User => hist.push_str(&format!("User: {}\n", msg.content)),
                        ChatRole::Iris => hist.push_str(&format!("Iris: {}\n", msg.content)),
                    }
                }
                hist
            };

            // Build current content section
            let content_section = if let Some(content) = &current_content {
                format!("\n## Current Content\n```\n{}\n```\n", content)
            } else {
                String::new()
            };

            // Tool-based update instructions
            let update_instructions = r"
## Response Guidelines
- Be concise - don't repeat content the user already sees
- When updating content, briefly explain what you changed

## Content Update Tools
You have tools to update content. When the user asks you to modify, change, update, or rewrite content:

1. **update_commit** - Update the commit message (emoji, title, message)
2. **update_pr** - Update the PR description (content)
3. **update_review** - Update the code review (review: complete structured review object). Preserve unmodified findings, metadata, evidence, and statistics from the current full review.

Simply call the appropriate tool with the new content. Do NOT echo back the full content in your response - the tool will update it directly.";

            let prompt = format!(
                "{}{}{}{}\n\n## Current Request\nUser: {}",
                mode_context, content_section, history_str, update_instructions, message
            );

            // Execute with streaming and content update tools
            let streaming_tx = tx.clone();
            let on_chunk = move |chunk: &str, aggregated: &str| {
                let _ = streaming_tx.send(IrisTaskResult::StreamingChunk {
                    task_type: TaskType::Chat,
                    chunk: chunk.to_string(),
                    aggregated: aggregated.to_string(),
                });
            };

            match agent
                .execute_chat_streaming(&prompt, content_tx, on_chunk)
                .await
            {
                Ok(response) => {
                    // Signal streaming complete
                    let _ = tx.send(IrisTaskResult::StreamingComplete {
                        task_type: TaskType::Chat,
                    });

                    let text = match response {
                        StructuredResponse::PlainText(text) => text,
                        other => other.to_string(),
                    };

                    tracing::debug!("Chat response received, length: {}", text.len());
                    let _ = tx.send(IrisTaskResult::ChatResponse(text));
                }
                Err(e) => {
                    let _ = tx.send(IrisTaskResult::ChatResponse(format!(
                        "I encountered an error: {}",
                        e
                    )));
                }
            }

            // Signal that we're done so the helper tasks stop
            cancel_token.cancel();
        });
    }

    /// Get ALL generated content for chat context (universal across modes)
    pub(super) fn get_current_content_for_chat(&self) -> Option<String> {
        use crate::studio::utils::truncate_chars;
        let mut sections = Vec::new();

        // Commit message
        let commit = &self.state.modes.commit;
        if let Some(msg) = commit.messages.get(commit.current_index) {
            let formatted = crate::types::format_commit_message(msg);
            if !formatted.trim().is_empty() {
                sections.push(format!("## Commit Message\n{}", formatted));
            }
        }

        // Code review
        let review = &self.state.modes.review.review_content;
        if !review.is_empty() {
            let preview = truncate_chars(review, 500);
            sections.push(format!("## Code Review\n{}", preview));
        }

        // PR description
        let pr = &self.state.modes.pr.pr_content;
        if !pr.is_empty() {
            let preview = truncate_chars(pr, 500);
            sections.push(format!("## PR Description\n{}", preview));
        }

        // Changelog
        let cl = &self.state.modes.changelog.changelog_content;
        if !cl.is_empty() {
            let preview = truncate_chars(cl, 500);
            sections.push(format!("## Changelog\n{}", preview));
        }

        // Release notes
        let rn = &self.state.modes.release_notes.release_notes_content;
        if !rn.is_empty() {
            let preview = truncate_chars(rn, 500);
            sections.push(format!("## Release Notes\n{}", preview));
        }

        if sections.is_empty() {
            None
        } else {
            Some(sections.join("\n\n"))
        }
    }

    pub(super) fn chat_content_context(&self, supplied: Option<String>) -> Option<String> {
        let mut content = supplied.or_else(|| self.get_current_content_for_chat());
        if let Some(review) = &self.state.modes.review.review {
            match serde_json::to_string(review) {
                Ok(review_json) => {
                    content.get_or_insert_with(String::new).push_str(&format!(
                        "\n\n## Current Full Review\nUse this complete structured review as the starting point for update_review.\n{review_json}"
                    ));
                }
                Err(error) => tracing::warn!("Could not serialize review chat context: {error}"),
            }
        }
        content
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Review Generation
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Spawn a task for code review generation
    pub(super) fn spawn_review_generation(&self, from_ref: String, to_ref: String) {
        use super::super::events::AgentTask;
        use crate::agents::{StructuredResponse, TaskContext};

        let task = AgentTask::Review {
            from_ref: from_ref.clone(),
            to_ref: to_ref.clone(),
        };

        self.spawn_structured_task(TaskType::Review, &task, move |agent| async move {
            let context = match TaskContext::for_review(None, Some(from_ref), Some(to_ref), false) {
                Ok(ctx) => ctx,
                Err(e) => {
                    return IrisTaskResult::Error {
                        task_type: TaskType::Review,
                        error: format!("Context error: {e}"),
                    };
                }
            };

            match agent.execute_task("review", context).await {
                Ok(response) => {
                    let review = match response {
                        StructuredResponse::Review(review) => review,
                        StructuredResponse::PlainText(summary) => {
                            crate::types::Review::from_unstructured(&summary)
                        }
                        other => crate::types::Review::from_unstructured(&other.to_string()),
                    };
                    IrisTaskResult::ReviewContent(review)
                }
                Err(e) => IrisTaskResult::Error {
                    task_type: TaskType::Review,
                    error: format!("Review error: {e}"),
                },
            }
        });
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // PR Generation
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Spawn a task for PR description generation
    pub(super) fn spawn_pr_generation(&self, base_branch: String, to_ref: &str) {
        use super::super::events::AgentTask;
        use crate::agents::{StructuredResponse, TaskContext};

        let to_ref = to_ref.to_string();
        let task = AgentTask::PR {
            base_branch: base_branch.clone(),
            to_ref: to_ref.clone(),
        };

        self.spawn_structured_task(TaskType::PR, &task, move |agent| async move {
            let context = TaskContext::for_pr(Some(base_branch), Some(to_ref));

            match agent.execute_task("pr", context).await {
                Ok(response) => {
                    let text = match response {
                        StructuredResponse::PullRequest(pr) => pr.content,
                        StructuredResponse::PlainText(t) => t,
                        other => other.to_string(),
                    };
                    IrisTaskResult::PRContent(text)
                }
                Err(e) => IrisTaskResult::Error {
                    task_type: TaskType::PR,
                    error: format!("PR error: {e}"),
                },
            }
        });
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Changelog Generation
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Spawn a task for changelog generation
    pub(super) fn spawn_changelog_generation(&self, from_ref: String, to_ref: String) {
        use super::super::events::AgentTask;
        use crate::agents::{StructuredResponse, TaskContext};

        let task = AgentTask::Changelog {
            from_ref: from_ref.clone(),
            to_ref: to_ref.clone(),
        };

        self.spawn_structured_task(TaskType::Changelog, &task, move |agent| async move {
            let context = TaskContext::for_changelog(from_ref, Some(to_ref), None, None);

            match agent.execute_task("changelog", context).await {
                Ok(response) => {
                    let text = match response {
                        StructuredResponse::Changelog(cl) => cl.content,
                        StructuredResponse::PlainText(t) => t,
                        other => other.to_string(),
                    };
                    IrisTaskResult::ChangelogContent(text)
                }
                Err(e) => IrisTaskResult::Error {
                    task_type: TaskType::Changelog,
                    error: format!("Changelog error: {e}"),
                },
            }
        });
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Release Notes Generation
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Spawn a task for release notes generation
    pub(super) fn spawn_release_notes_generation(&self, from_ref: String, to_ref: String) {
        use super::super::events::AgentTask;
        use crate::agents::{StructuredResponse, TaskContext};

        let task = AgentTask::ReleaseNotes {
            from_ref: from_ref.clone(),
            to_ref: to_ref.clone(),
        };

        self.spawn_structured_task(TaskType::ReleaseNotes, &task, move |agent| async move {
            let context = TaskContext::for_changelog(from_ref, Some(to_ref), None, None);

            match agent.execute_task("release_notes", context).await {
                Ok(response) => {
                    let text = match response {
                        StructuredResponse::ReleaseNotes(rn) => rn.content,
                        StructuredResponse::PlainText(t) => t,
                        other => other.to_string(),
                    };
                    IrisTaskResult::ReleaseNotesContent(text)
                }
                Err(e) => IrisTaskResult::Error {
                    task_type: TaskType::ReleaseNotes,
                    error: format!("Release notes error: {e}"),
                },
            }
        });
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Commit Generation
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Spawn a task to generate a commit message
    pub(super) fn spawn_commit_generation(
        &self,
        instructions: Option<String>,
        preset: String,
        use_gitmoji: Option<bool>,
        amend: bool,
    ) {
        use super::super::events::AgentTask;
        use crate::agents::{StructuredResponse, TaskContext};

        let Some(agent) = self.agent_service.clone() else {
            let tx = self.iris_result_tx.clone();
            let _ = tx.send(IrisTaskResult::Error {
                task_type: TaskType::Commit,
                error: "Agent service not available".to_string(),
            });
            return;
        };

        // Spawn dynamic status messages
        let task = AgentTask::Commit {
            instructions: instructions.clone(),
            preset: preset.clone(),
            use_gitmoji,
            amend,
        };
        self.spawn_status_messages(&task);

        // Get original message for amend mode
        let original_message = if amend {
            self.state
                .repo
                .as_ref()
                .and_then(|r| r.get_head_commit_message().ok())
                .unwrap_or_default()
        } else {
            String::new()
        };

        let tx = self.iris_result_tx.clone();

        tokio::spawn(async move {
            // Use amend context if amending, otherwise standard commit context
            let context = if amend {
                TaskContext::for_amend(original_message)
            } else {
                TaskContext::for_gen()
            };

            // Execute commit capability with style overrides
            let preset_opt = if preset == "default" {
                None
            } else {
                Some(preset.as_str())
            };

            match agent
                .execute_task_with_style(
                    "commit",
                    context,
                    preset_opt,
                    use_gitmoji,
                    instructions.as_deref(),
                )
                .await
            {
                Ok(response) => {
                    // Extract message from response
                    match response {
                        StructuredResponse::CommitMessage(msg) => {
                            let _ = tx.send(IrisTaskResult::CommitMessages(vec![msg]));
                        }
                        _ => {
                            let _ = tx.send(IrisTaskResult::Error {
                                task_type: TaskType::Commit,
                                error: "Unexpected response type from agent".to_string(),
                            });
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(IrisTaskResult::Error {
                        task_type: TaskType::Commit,
                        error: format!("Agent error: {}", e),
                    });
                }
            }
        });
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // Semantic Blame
    // ═══════════════════════════════════════════════════════════════════════════════

    /// Gather blame information from git and spawn the semantic blame agent.
    /// All blocking I/O (file read, git blame) runs in a background task to avoid
    /// blocking the UI event loop.
    pub(super) fn gather_blame_and_spawn(
        &self,
        file: &std::path::Path,
        start_line: usize,
        end_line: usize,
    ) {
        use crate::agents::StructuredResponse;

        let Some(repo) = &self.state.repo else {
            let tx = self.iris_result_tx.clone();
            let _ = tx.send(IrisTaskResult::Error {
                task_type: TaskType::SemanticBlame,
                error: "Repository not available".to_string(),
            });
            return;
        };

        let Some(agent) = self.agent_service.clone() else {
            let tx = self.iris_result_tx.clone();
            let _ = tx.send(IrisTaskResult::Error {
                task_type: TaskType::SemanticBlame,
                error: "Agent service not available".to_string(),
            });
            return;
        };

        // Clone values needed in the async task
        let tx = self.iris_result_tx.clone();
        let file = file.to_path_buf();
        let repo_path = repo.repo_path().clone();

        tokio::spawn(async move {
            // Run blocking I/O in spawn_blocking to avoid blocking the tokio runtime
            let blame_result = tokio::task::spawn_blocking(move || {
                use std::fs;
                use std::process::Command;

                // Read file content
                let content = fs::read_to_string(&file)?;
                let lines: Vec<&str> = content.lines().collect();

                if start_line == 0 || start_line > lines.len() {
                    return Err(anyhow::anyhow!("Invalid line range"));
                }

                let end = end_line.min(lines.len());
                let code_content = lines[(start_line - 1)..end].join("\n");

                // Run git blame
                let output = Command::new("git")
                    .args([
                        "-C",
                        &repo_path.to_string_lossy(),
                        "blame",
                        "-L",
                        &format!("{},{}", start_line, end_line),
                        "--porcelain",
                        &file.to_string_lossy(),
                    ])
                    .output()?;

                if !output.status.success() {
                    let err = String::from_utf8_lossy(&output.stderr);
                    return Err(anyhow::anyhow!("Git blame failed: {}", err));
                }

                let blame_output = String::from_utf8_lossy(&output.stdout);
                let (commit_hash, author, commit_date, commit_message) =
                    parse_blame_porcelain(&blame_output);

                Ok(BlameInfo {
                    file,
                    start_line,
                    end_line,
                    commit_hash,
                    author,
                    commit_date,
                    commit_message,
                    code_content,
                })
            })
            .await;

            // Handle spawn_blocking result
            let blame_info = match blame_result {
                Ok(Ok(info)) => info,
                Ok(Err(e)) => {
                    let _ = tx.send(IrisTaskResult::Error {
                        task_type: TaskType::SemanticBlame,
                        error: e.to_string(),
                    });
                    return;
                }
                Err(e) => {
                    let _ = tx.send(IrisTaskResult::Error {
                        task_type: TaskType::SemanticBlame,
                        error: format!("Task panicked: {}", e),
                    });
                    return;
                }
            };

            // Build context for agent
            let context_text = format!(
                "File: {}\nLines: {}-{}\nCommit: {} by {} on {}\nMessage: {}\n\nCode:\n{}",
                blame_info.file.display(),
                blame_info.start_line,
                blame_info.end_line,
                blame_info.commit_hash,
                blame_info.author,
                blame_info.commit_date,
                blame_info.commit_message,
                blame_info.code_content
            );

            // Execute semantic_blame capability
            match agent
                .execute_task_with_prompt("semantic_blame", &context_text)
                .await
            {
                Ok(response) => match response {
                    StructuredResponse::SemanticBlame(explanation) => {
                        let result = SemanticBlameResult {
                            file: blame_info.file,
                            start_line: blame_info.start_line,
                            end_line: blame_info.end_line,
                            commit_hash: blame_info.commit_hash,
                            author: blame_info.author,
                            commit_date: blame_info.commit_date,
                            commit_message: blame_info.commit_message,
                            explanation,
                        };
                        let _ = tx.send(IrisTaskResult::SemanticBlame(result));
                    }
                    _ => {
                        let _ = tx.send(IrisTaskResult::Error {
                            task_type: TaskType::SemanticBlame,
                            error: "Unexpected response type from agent".to_string(),
                        });
                    }
                },
                Err(e) => {
                    let _ = tx.send(IrisTaskResult::Error {
                        task_type: TaskType::SemanticBlame,
                        error: format!("Semantic blame error: {}", e),
                    });
                }
            }
        });
    }

    /// Spawn the semantic blame agent to explain why the code exists.
    /// Used when blame info is already collected (e.g., from `AgentTask::SemanticBlame`).
    pub(super) fn spawn_semantic_blame(&self, blame_info: BlameInfo) {
        use super::super::events::AgentTask;
        use crate::agents::StructuredResponse;

        let Some(agent) = self.agent_service.clone() else {
            let tx = self.iris_result_tx.clone();
            let _ = tx.send(IrisTaskResult::Error {
                task_type: TaskType::SemanticBlame,
                error: "Agent service not available".to_string(),
            });
            return;
        };

        // Spawn dynamic status messages
        let task = AgentTask::SemanticBlame {
            blame_info: blame_info.clone(),
        };
        self.spawn_status_messages(&task);

        let tx = self.iris_result_tx.clone();

        tokio::spawn(async move {
            // Build context with blame info
            let context_text = format!(
                "File: {}\nLines: {}-{}\nCommit: {} by {} on {}\nMessage: {}\n\nCode:\n{}",
                blame_info.file.display(),
                blame_info.start_line,
                blame_info.end_line,
                blame_info.commit_hash,
                blame_info.author,
                blame_info.commit_date,
                blame_info.commit_message,
                blame_info.code_content
            );

            // Execute semantic_blame capability
            match agent
                .execute_task_with_prompt("semantic_blame", &context_text)
                .await
            {
                Ok(response) => match response {
                    StructuredResponse::SemanticBlame(explanation) => {
                        let result = SemanticBlameResult {
                            file: blame_info.file,
                            start_line: blame_info.start_line,
                            end_line: blame_info.end_line,
                            commit_hash: blame_info.commit_hash,
                            author: blame_info.author,
                            commit_date: blame_info.commit_date,
                            commit_message: blame_info.commit_message,
                            explanation,
                        };
                        let _ = tx.send(IrisTaskResult::SemanticBlame(result));
                    }
                    _ => {
                        let _ = tx.send(IrisTaskResult::Error {
                            task_type: TaskType::SemanticBlame,
                            error: "Unexpected response type from agent".to_string(),
                        });
                    }
                },
                Err(e) => {
                    let _ = tx.send(IrisTaskResult::Error {
                        task_type: TaskType::SemanticBlame,
                        error: format!("Semantic blame error: {}", e),
                    });
                }
            }
        });
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Helper Functions
// ═══════════════════════════════════════════════════════════════════════════════

pub(super) async fn forward_content_updates(
    mut receiver: crate::agents::tools::ContentUpdateReceiver,
    sender: tokio::sync::mpsc::UnboundedSender<IrisTaskResult>,
    completion: tokio_util::sync::CancellationToken,
) {
    use crate::agents::tools::ContentUpdate;

    loop {
        tokio::select! {
            // A completed generation can still have successful tool updates queued.
            biased;
            update = receiver.recv() => {
                let Some(update) = update else { break };
                let update = match update {
                    ContentUpdate::Commit { emoji, title, message } => {
                        ChatUpdateType::CommitMessage(GeneratedMessage {
                            emoji, title, message, completion_message: None,
                        })
                    }
                    ContentUpdate::PR { content } => ChatUpdateType::PRDescription(content),
                    ContentUpdate::Review { review } => ChatUpdateType::Review(review),
                };
                if sender.send(IrisTaskResult::ChatUpdate(update)).is_err() {
                    break;
                }
            }
            () = completion.cancelled() => break,
        }
    }
}

/// Parse git blame porcelain output to extract commit info
fn parse_blame_porcelain(output: &str) -> (String, String, String, String) {
    let mut commit_hash = String::new();
    let mut author = String::new();
    let mut commit_time = String::new();
    let mut summary = String::new();

    for line in output.lines() {
        if commit_hash.is_empty()
            && line.len() >= 40
            && line.chars().take(40).all(|c| c.is_ascii_hexdigit())
        {
            commit_hash = line.split_whitespace().next().unwrap_or("").to_string();
        } else if let Some(rest) = line.strip_prefix("author ") {
            author = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("author-time ") {
            if let Ok(timestamp) = rest.parse::<i64>() {
                commit_time = chrono::DateTime::from_timestamp(timestamp, 0).map_or_else(
                    || "Unknown date".to_string(),
                    |dt| dt.format("%Y-%m-%d %H:%M").to_string(),
                );
            }
        } else if let Some(rest) = line.strip_prefix("summary ") {
            summary = rest.to_string();
        }
    }

    if commit_hash.is_empty() {
        commit_hash = "Unknown".to_string();
    }
    if author.is_empty() {
        author = "Unknown".to_string();
    }
    if commit_time.is_empty() {
        commit_time = "Unknown date".to_string();
    }

    (commit_hash, author, commit_time, summary)
}
