//! Tests for the Reducer

use crate::config::Config;
use crate::studio::events::{
    AgentResult, AgentTask, ContentPayload, ContentType, NotificationLevel, SideEffect,
    StudioEvent, TaskType,
};
use crate::studio::history::History;
use crate::studio::reducer::reduce;
use crate::studio::state::{Mode, PanelId, StudioState};
use crate::types::{GeneratedMessage, Review, ReviewMetadata, ReviewStats};

fn test_state() -> StudioState {
    StudioState::new(Config::default(), None)
}

#[test]
fn file_log_results_only_update_the_selected_file() {
    use crate::studio::state::FileLogEntry;
    let mut state = test_state();
    let mut history = History::new();
    state.modes.explore.current_file = Some("new.rs".into());
    state.modes.explore.file_log_loading = true;
    let entries = vec![FileLogEntry {
        hash: "abc123".into(),
        message: "correct history".into(),
        author: "Author".into(),
        short_hash: "abc123".into(),
        relative_time: "now".into(),
        additions: None,
        deletions: None,
    }];
    reduce(
        &mut state,
        StudioEvent::FileLogLoaded {
            file: "old.rs".into(),
            entries: entries.clone(),
        },
        &mut history,
    );
    assert!(state.modes.explore.file_log.is_empty());
    assert!(state.modes.explore.file_log_loading);
    reduce(
        &mut state,
        StudioEvent::FileLogLoaded {
            file: "new.rs".into(),
            entries,
        },
        &mut history,
    );
    assert_eq!(state.modes.explore.file_log.len(), 1);
    assert!(!state.modes.explore.file_log_loading);
}

#[test]
fn file_log_failures_only_stop_loading_for_the_current_selection() {
    let mut state = test_state();
    let mut history = History::new();
    state.modes.explore.current_file = Some("new.rs".into());
    state.modes.explore.file_log_loading = true;
    for (file, still_loading) in [("old.rs", true), ("new.rs", false)] {
        reduce(
            &mut state,
            StudioEvent::FileLogFailed {
                file: file.into(),
                error: "git failed".into(),
            },
            &mut history,
        );
        assert_eq!(state.modes.explore.file_log_loading, still_loading);
    }
}

fn test_review(summary: &str) -> Review {
    Review {
        summary: summary.to_string(),
        metadata: ReviewMetadata::default(),
        findings: Vec::new(),
        stats: ReviewStats::default(),
        parse_failed: false,
    }
}

#[test]
fn test_mode_switch() {
    let mut state = test_state();
    let mut history = History::new();

    let effects = reduce(
        &mut state,
        StudioEvent::SwitchMode(Mode::Commit),
        &mut history,
    );

    assert_eq!(state.active_mode, Mode::Commit);
    assert!(!effects.is_empty()); // Should have LoadData effect
}

#[test]
fn test_focus_panel() {
    let mut state = test_state();
    let mut history = History::new();

    let _ = reduce(
        &mut state,
        StudioEvent::FocusPanel(PanelId::Right),
        &mut history,
    );

    assert_eq!(state.focused_panel, PanelId::Right);
}

#[test]
fn test_notify() {
    let mut state = test_state();
    let mut history = History::new();

    let _ = reduce(
        &mut state,
        StudioEvent::Notify {
            level: NotificationLevel::Success,
            message: "Test notification".to_string(),
        },
        &mut history,
    );

    assert!(!state.notifications.is_empty());
}

#[test]
fn test_quit_produces_effect() {
    let mut state = test_state();
    let mut history = History::new();

    let effects = reduce(&mut state, StudioEvent::Quit, &mut history);

    assert!(effects.iter().any(|e| matches!(e, SideEffect::Quit)));
}

#[test]
fn test_agent_result_commit_messages() {
    let mut state = test_state();
    let mut history = History::new();
    state.active_mode = Mode::Commit;

    let messages = vec![
        GeneratedMessage {
            emoji: Some("✨".to_string()),
            title: "Add feature".to_string(),
            message: "Details here".to_string(),
            completion_message: None,
        },
        GeneratedMessage {
            emoji: Some("🐛".to_string()),
            title: "Fix bug".to_string(),
            message: "More details".to_string(),
            completion_message: None,
        },
    ];

    let _ = reduce(
        &mut state,
        StudioEvent::AgentComplete {
            task_type: TaskType::Commit,
            result: AgentResult::CommitMessages(messages.clone()),
        },
        &mut history,
    );

    assert_eq!(state.modes.commit.messages.len(), 2);
    assert_eq!(state.modes.commit.messages[0].title, "Add feature");
    assert!(!state.modes.commit.generating);
}

#[test]
fn test_agent_result_review_content() {
    let mut state = test_state();
    let mut history = History::new();
    state.active_mode = Mode::Review;
    state.modes.review.generating = true;

    let review = test_review("Looks good!");
    let expected = review.raw_content();

    let _ = reduce(
        &mut state,
        StudioEvent::AgentComplete {
            task_type: TaskType::Review,
            result: AgentResult::ReviewContent(review.clone()),
        },
        &mut history,
    );

    assert_eq!(state.modes.review.review_content, expected);
    assert_eq!(
        state
            .modes
            .review
            .review
            .as_ref()
            .map(|stored| &stored.summary),
        Some(&review.summary)
    );
    assert!(!state.modes.review.generating);
}

#[test]
fn test_message_variant_navigation() {
    let mut state = test_state();
    let mut history = History::new();
    state.active_mode = Mode::Commit;

    // Set up multiple messages
    state.modes.commit.messages = vec![
        GeneratedMessage {
            emoji: None,
            title: "First".to_string(),
            message: String::new(),
            completion_message: None,
        },
        GeneratedMessage {
            emoji: None,
            title: "Second".to_string(),
            message: String::new(),
            completion_message: None,
        },
        GeneratedMessage {
            emoji: None,
            title: "Third".to_string(),
            message: String::new(),
            completion_message: None,
        },
    ];
    state
        .modes
        .commit
        .message_editor
        .set_messages(state.modes.commit.messages.clone());

    // Navigate forward
    let _ = reduce(&mut state, StudioEvent::NextMessageVariant, &mut history);
    assert_eq!(state.modes.commit.current_index, 1);

    let _ = reduce(&mut state, StudioEvent::NextMessageVariant, &mut history);
    assert_eq!(state.modes.commit.current_index, 2);

    // Wrap around
    let _ = reduce(&mut state, StudioEvent::NextMessageVariant, &mut history);
    assert_eq!(state.modes.commit.current_index, 0);

    // Navigate backward (wraps to end)
    let _ = reduce(&mut state, StudioEvent::PrevMessageVariant, &mut history);
    assert_eq!(state.modes.commit.current_index, 2);
}

#[test]
fn test_copy_to_clipboard_effect() {
    let mut state = test_state();
    let mut history = History::new();

    let text = "Copy this!".to_string();
    let effects = reduce(
        &mut state,
        StudioEvent::CopyToClipboard(text.clone()),
        &mut history,
    );

    assert!(
        effects
            .iter()
            .any(|e| matches!(e, SideEffect::CopyToClipboard(t) if t == &text))
    );
}

#[test]
fn test_toggle_edit_mode() {
    let mut state = test_state();
    let mut history = History::new();
    state.active_mode = Mode::Commit;

    assert!(!state.modes.commit.editing_message);

    let _ = reduce(&mut state, StudioEvent::ToggleEditMode, &mut history);
    assert!(state.modes.commit.editing_message);

    let _ = reduce(&mut state, StudioEvent::ToggleEditMode, &mut history);
    assert!(!state.modes.commit.editing_message);
}

#[test]
fn test_generate_commit_produces_agent_effect() {
    let mut state = test_state();
    let mut history = History::new();
    state.active_mode = Mode::Commit;

    let effects = reduce(
        &mut state,
        StudioEvent::GenerateCommit {
            instructions: None,
            preset: "default".to_string(),
            use_gitmoji: true,
            amend: false,
        },
        &mut history,
    );

    assert!(state.modes.commit.generating);
    assert!(effects.iter().any(|e| matches!(
        e,
        SideEffect::SpawnAgent {
            task: AgentTask::Commit { .. }
        }
    )));
}

#[test]
fn test_update_content_sets_review_content() {
    let mut state = test_state();
    let mut history = History::new();
    state.active_mode = Mode::Review;

    let _ = reduce(
        &mut state,
        StudioEvent::UpdateContent {
            content_type: ContentType::CodeReview,
            content: ContentPayload::Markdown("## Final Review\n\nLooks good.".to_string()),
        },
        &mut history,
    );

    assert_eq!(
        state.modes.review.review_content,
        "## Final Review\n\nLooks good."
    );
}

#[test]
fn test_streaming_chunk_ignores_review_panel_content() {
    let mut state = test_state();
    let mut history = History::new();
    state.active_mode = Mode::Review;

    let _ = reduce(
        &mut state,
        StudioEvent::StreamingChunk {
            task_type: TaskType::Review,
            chunk: "{".to_string(),
            aggregated: "{\"content\":\"raw stream\"}".to_string(),
        },
        &mut history,
    );

    assert!(state.modes.review.review_content.is_empty());
}

#[test]
fn test_streaming_chunk_updates_chat_response() {
    let mut state = test_state();
    let mut history = History::new();

    let _ = reduce(
        &mut state,
        StudioEvent::StreamingChunk {
            task_type: TaskType::Chat,
            chunk: "hello".to_string(),
            aggregated: "hello world".to_string(),
        },
        &mut history,
    );

    assert_eq!(
        state.chat_state.streaming_response.as_deref(),
        Some("hello world")
    );
}

#[test]
fn test_streaming_complete_clears_chat_response() {
    let mut state = test_state();
    let mut history = History::new();
    state.chat_state.streaming_response = Some("hello world".to_string());

    let _ = reduce(
        &mut state,
        StudioEvent::StreamingComplete {
            task_type: TaskType::Chat,
        },
        &mut history,
    );

    assert_eq!(state.chat_state.streaming_response, None);
}

#[test]
fn test_agent_error_clears_generating_flag() {
    let mut state = test_state();
    let mut history = History::new();
    state.active_mode = Mode::Commit;
    state.modes.commit.generating = true;

    let _ = reduce(
        &mut state,
        StudioEvent::AgentError {
            task_type: TaskType::Commit,
            error: "Something failed".to_string(),
        },
        &mut history,
    );

    assert!(!state.modes.commit.generating);
    // Should have a notification
    assert!(!state.notifications.is_empty());
}
