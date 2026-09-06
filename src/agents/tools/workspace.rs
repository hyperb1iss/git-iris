//! Iris workspace tool for Rig
//!
//! This tool provides Iris with her own personal workspace for taking notes,
//! creating task lists, and managing her workflow during complex operations.

use anyhow::Result;
use rig::tool::portable::PortableTool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::{Arc, Mutex};

use super::common::parameters_schema;

// Use standard tool error macro for consistency
crate::define_tool_error!(WorkspaceError);

/// Workspace tool for Iris's note-taking and task management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    #[serde(skip)]
    data: Arc<Mutex<WorkspaceData>>,
}

#[derive(Debug, Default)]
struct WorkspaceData {
    notes: Vec<String>,
    tasks: Vec<WorkspaceTask>,
}

#[derive(Debug, Clone)]
struct WorkspaceTask {
    description: String,
    status: TaskStatus,
    priority: TaskPriority,
}

/// Workspace action type
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceAction {
    /// Add a note to the workspace
    AddNote,
    /// Add a task to track
    AddTask,
    /// Update an existing task's status
    UpdateTask,
    /// Get summary of notes and tasks
    #[default]
    GetSummary,
}

/// Task priority level
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum TaskPriority {
    Low,
    #[default]
    Medium,
    High,
    Critical,
}

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
    Blocked,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::InProgress => write!(f, "in_progress"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Blocked => write!(f, "blocked"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WorkspaceArgs {
    /// Action to perform
    pub action: WorkspaceAction,
    /// Content for note or task description
    #[serde(default)]
    pub content: Option<String>,
    /// Priority level for tasks
    #[serde(default)]
    pub priority: Option<TaskPriority>,
    /// Index of task to update (0-based)
    #[serde(default)]
    pub task_index: Option<usize>,
    /// New status for task updates
    #[serde(default)]
    pub status: Option<TaskStatus>,
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}

impl Workspace {
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(WorkspaceData::default())),
        }
    }
}

impl PortableTool for Workspace {
    const NAME: &'static str = "workspace";
    type Error = WorkspaceError;
    type Args = WorkspaceArgs;
    type Output = String;

    fn description(&self) -> String {
        "Iris's personal workspace for notes and task management. Use this to track progress, take notes on findings, and manage complex workflows.".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        parameters_schema::<WorkspaceArgs>()
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "Defer synchronous tool work until polling inside the repository context"
    )]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let mut data = self
            .data
            .lock()
            .map_err(|e| WorkspaceError(e.to_string()))?;

        let result = match args.action {
            WorkspaceAction::AddNote => {
                let content = args
                    .content
                    .ok_or_else(|| WorkspaceError("Content required for add_note".to_string()))?;
                data.notes.push(content);
                json!({
                    "success": true,
                    "message": "Note added successfully",
                    "note_count": data.notes.len()
                })
            }
            WorkspaceAction::AddTask => {
                let content = args
                    .content
                    .ok_or_else(|| WorkspaceError("Content required for add_task".to_string()))?;
                let priority = args.priority.unwrap_or_default();

                data.tasks.push(WorkspaceTask {
                    description: content,
                    status: TaskStatus::Pending,
                    priority,
                });

                json!({
                    "success": true,
                    "message": "Task added successfully",
                    "task_count": data.tasks.len()
                })
            }
            WorkspaceAction::UpdateTask => {
                let task_index = args.task_index.ok_or_else(|| {
                    WorkspaceError("task_index required for update_task".to_string())
                })?;
                let status = args
                    .status
                    .ok_or_else(|| WorkspaceError("status required for update_task".to_string()))?;

                if task_index >= data.tasks.len() {
                    return Err(WorkspaceError(format!(
                        "Task index {task_index} out of range"
                    )));
                }

                data.tasks[task_index].status = status.clone();

                json!({
                    "success": true,
                    "message": format!("Task {} updated to {}", task_index, status),
                })
            }
            WorkspaceAction::GetSummary => {
                let pending = data
                    .tasks
                    .iter()
                    .filter(|t| matches!(t.status, TaskStatus::Pending))
                    .count();
                let in_progress = data
                    .tasks
                    .iter()
                    .filter(|t| matches!(t.status, TaskStatus::InProgress))
                    .count();
                let completed = data
                    .tasks
                    .iter()
                    .filter(|t| matches!(t.status, TaskStatus::Completed))
                    .count();

                json!({
                    "notes_count": data.notes.len(),
                    "tasks": {
                        "total": data.tasks.len(),
                        "pending": pending,
                        "in_progress": in_progress,
                        "completed": completed
                    },
                    "recent_notes": data.notes.iter().rev().take(3).collect::<Vec<_>>(),
                    "active_tasks": data.tasks.iter()
                        .filter(|t| !matches!(t.status, TaskStatus::Completed))
                        .map(|t| json!({
                            "description": t.description,
                            "status": t.status.to_string(),
                            "priority": format!("{:?}", t.priority).to_lowercase()
                        }))
                        .collect::<Vec<_>>()
                })
            }
        };

        serde_json::to_string_pretty(&result).map_err(|e| WorkspaceError(e.to_string()))
    }
}
