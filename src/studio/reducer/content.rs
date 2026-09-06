//! Content update event handlers
//!
//! Handles tool-triggered content updates (from chat tools updating UI).

use super::super::events::{ContentPayload, ContentType, EventSource, SideEffect};
use super::super::history::{ContentData, History};
use super::super::state::{Mode, Notification, StudioState};

/// Handle `UpdateContent` event (tool-triggered)
pub fn update_content(
    state: &mut StudioState,
    history: &mut History,
    content_type: ContentType,
    content: ContentPayload,
) {
    match (content_type, content) {
        (ContentType::CommitMessage, ContentPayload::Commit(msg)) => {
            // Update current message
            if state.modes.commit.messages.is_empty() {
                state.modes.commit.messages = vec![msg.clone()];
                state
                    .modes
                    .commit
                    .message_editor
                    .set_messages(vec![msg.clone()]);
            } else {
                let idx = state.modes.commit.current_index;
                state.modes.commit.messages[idx] = msg.clone();
                state
                    .modes
                    .commit
                    .message_editor
                    .set_messages(state.modes.commit.messages.clone());
            }

            history.record_content(
                Mode::Commit,
                content_type,
                &ContentData::Commit(msg),
                EventSource::Tool,
                "tool_update",
            );
        }

        (ContentType::PRDescription, ContentPayload::Markdown(content)) => {
            state.modes.pr.pr_content.clone_from(&content);

            history.record_content(
                Mode::PR,
                content_type,
                &ContentData::Markdown(content),
                EventSource::Tool,
                "tool_update",
            );
        }

        (ContentType::CodeReview, ContentPayload::Review(review)) => {
            let content = review.raw_content();
            state.modes.review.review = Some(*review);
            state.modes.review.review_content.clone_from(&content);
            state.modes.review.review_scroll = 0;

            history.record_content(
                Mode::Review,
                content_type,
                &ContentData::Markdown(content),
                EventSource::Tool,
                "tool_update",
            );
        }

        (ContentType::CodeReview, ContentPayload::Markdown(content)) => {
            state.modes.review.reset_review();
            state.modes.review.review_content.clone_from(&content);

            history.record_content(
                Mode::Review,
                content_type,
                &ContentData::Markdown(content),
                EventSource::Tool,
                "tool_update",
            );
        }

        (ContentType::Changelog, ContentPayload::Markdown(content)) => {
            state.modes.changelog.changelog_content.clone_from(&content);

            history.record_content(
                Mode::Changelog,
                content_type,
                &ContentData::Markdown(content),
                EventSource::Tool,
                "tool_update",
            );
        }

        (ContentType::ReleaseNotes, ContentPayload::Markdown(content)) => {
            state
                .modes
                .release_notes
                .release_notes_content
                .clone_from(&content);

            history.record_content(
                Mode::ReleaseNotes,
                content_type,
                &ContentData::Markdown(content),
                EventSource::Tool,
                "tool_update",
            );
        }

        _ => {
            // Mismatched content type and payload
            state.notify(Notification::warning("Received mismatched content update"));
        }
    }

    state.mark_dirty();
}

/// Handle file staging event
pub fn stage_file(path: std::path::PathBuf) -> SideEffect {
    SideEffect::GitStage(path)
}

/// Handle file unstaging event
pub fn unstage_file(path: std::path::PathBuf) -> SideEffect {
    SideEffect::GitUnstage(path)
}
