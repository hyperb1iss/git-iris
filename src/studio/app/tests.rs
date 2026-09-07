use super::*;
use crate::agents::tools::content_update::{
    ContentUpdate, UpdateCommitArgs, UpdateCommitTool, UpdatePRArgs, UpdatePRTool,
    UpdateReviewArgs, UpdateReviewTool, create_content_update_channel,
};
use rig::tool::portable::PortableTool;
use serde_json::json;

fn app() -> StudioApp {
    StudioApp::new(Config::default(), None, None, None)
}

fn review_fixture() -> Review {
    serde_json::from_value(json!({
        "summary": "Original review",
        "metadata": {"risk_level": "high", "strategy": "Inspect boundaries", "coverage_notes": ["src/auth.rs"]},
        "findings": [{
            "id": "R1", "severity": "high", "confidence": 95,
            "file": "src/auth.rs", "start_line": 10, "end_line": 12,
            "category": "security", "title": "Authorization bypass", "body": "Retain this finding.",
            "suggested_fix": "Check access before reading the record.",
            "evidence": [{"file": "src/routes.rs", "line": 20, "end_line": 22, "note": "Caller lacks an access check."}]
        }, {
            "id": "R2", "severity": "low", "confidence": 40,
            "file": "src/log.rs", "start_line": 3, "end_line": 3,
            "category": "other", "title": "Hidden finding", "body": "Retain lower confidence evidence too."
        }],
        "stats": {"files_reviewed": 2, "findings_count": 2, "high_count": 1, "low_count": 1}
    }))
    .expect("review fixture")
}

#[test]
fn chat_previews_handle_multibyte_characters_at_the_old_byte_boundary() {
    let mut app = app();
    let content = format!("{}🌸{}", "a".repeat(499), "z".repeat(20));
    app.state.modes.review.review_content.clone_from(&content);
    app.state.modes.pr.pr_content.clone_from(&content);
    app.state
        .modes
        .changelog
        .changelog_content
        .clone_from(&content);
    app.state.modes.release_notes.release_notes_content = content;

    let snapshot = app.get_current_content_for_chat().expect("content");
    for section in snapshot.split("\n\n") {
        let (_, preview) = section.split_once('\n').expect("section header");
        assert_eq!(preview.chars().count(), 500);
        assert!(preview.ends_with("..."));
    }
    assert_eq!(snapshot.split("\n\n").count(), 4);
}

#[test]
fn chat_keeps_full_structured_review_even_with_explicit_content() {
    let mut app = app();
    let mut review = review_fixture();
    review.summary = "🌸".repeat(600);
    let serialized = serde_json::to_string(&review).expect("serialize review");
    app.state.modes.review.review_content = review.raw_content();
    app.state.modes.review.review = Some(review);
    for supplied in [None, Some("Supplied context".to_string())] {
        let context = app.chat_content_context(supplied.clone()).expect("context");
        assert!(context.contains(&serialized));
        assert!(context.contains("Hidden finding"));
        if let Some(supplied) = supplied {
            assert!(context.contains(&supplied));
        }
    }
}

#[tokio::test]
async fn review_tool_update_preserves_findings_through_events_and_reducer() {
    let mut app = app();
    let original = review_fixture();
    app.state.modes.review.review = Some(original.clone());
    app.state.modes.review.review_content = original.raw_content();
    app.state.modes.review.review_scroll = 100;
    let mut updated = original.clone();
    updated.summary = "Updated summary".to_string();

    let (sender, receiver) = create_content_update_channel();
    let result = UpdateReviewTool::new(sender.clone())
        .call(UpdateReviewArgs {
            review: updated.clone(),
        })
        .await
        .expect("review update");
    assert!(result.contains("Review updated successfully"));
    let completion = tokio_util::sync::CancellationToken::new();
    completion.cancel();
    tokio::time::timeout(
        std::time::Duration::from_secs(1),
        agent_tasks::forward_content_updates(receiver, app.iris_result_tx.clone(), completion),
    )
    .await
    .expect("queued update must drain and forwarder must finish");
    app.check_iris_results();
    assert!(app.process_events().is_none());

    let actual = app
        .state
        .modes
        .review
        .review
        .as_ref()
        .expect("structured review");
    assert_eq!(actual.summary, updated.summary);
    assert_eq!(actual.findings, original.findings);
    assert_eq!(actual.metadata, original.metadata);
    assert_eq!(actual.stats, original.stats);
    assert_eq!(app.state.modes.review.review_content, updated.raw_content());
    assert_eq!(app.state.modes.review.review_scroll, 0);
    assert_eq!(
        app.history
            .content_version_count(Mode::Review, ContentType::CodeReview),
        1
    );
}

#[tokio::test]
async fn commit_and_pr_tools_keep_existing_payload_shapes() {
    let (sender, mut receiver) = create_content_update_channel();
    UpdateCommitTool::new(sender.clone())
        .call(
            serde_json::from_value::<UpdateCommitArgs>(json!({"title": "Commit title"}))
                .expect("commit args"),
        )
        .await
        .expect("commit update");
    assert!(
        matches!(receiver.recv().await, Some(ContentUpdate::Commit { title, message, emoji }) if title == "Commit title" && message.is_empty() && emoji.is_none())
    );
    UpdatePRTool::new(sender)
        .call(UpdatePRArgs {
            content: "PR markdown".to_string(),
        })
        .await
        .expect("PR update");
    assert!(
        matches!(receiver.recv().await, Some(ContentUpdate::PR { content }) if content == "PR markdown")
    );
}

#[test]
fn review_tool_schema_requires_structured_review() {
    let (sender, _receiver) = create_content_update_channel();
    let tool = UpdateReviewTool::new(sender);
    let schema = tool.parameters();
    assert_eq!(schema["required"], json!(["review"]));
    assert!(schema["properties"].get("content").is_none());
    assert!(serde_json::from_value::<UpdateReviewArgs>(json!({"content": "markdown"})).is_err());
    assert!(tool.description().contains("preserve unmodified findings"));
}

#[tokio::test]
async fn completed_chat_drains_queued_updates_in_order() {
    let (sender, receiver) = create_content_update_channel();
    for content in ["first", "second", "third"] {
        UpdatePRTool::new(sender.clone())
            .call(UpdatePRArgs {
                content: content.to_string(),
            })
            .await
            .expect("successful tool update");
    }
    let (result_tx, mut result_rx) = tokio::sync::mpsc::unbounded_channel();
    let completion = tokio_util::sync::CancellationToken::new();
    completion.cancel();
    tokio::time::timeout(
        std::time::Duration::from_secs(1),
        agent_tasks::forward_content_updates(receiver, result_tx, completion),
    )
    .await
    .expect("completed chat must drain updates and finish");
    for expected in ["first", "second", "third"] {
        assert!(
            matches!(result_rx.recv().await, Some(IrisTaskResult::ChatUpdate(ChatUpdateType::PRDescription(content))) if content == expected)
        );
    }
    assert!(result_rx.recv().await.is_none());
}

#[tokio::test]
async fn content_forwarder_exits_when_tool_channel_closes() {
    let (sender, receiver) = create_content_update_channel();
    drop(sender);
    let (result_tx, mut result_rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::time::timeout(
        std::time::Duration::from_secs(1),
        agent_tasks::forward_content_updates(
            receiver,
            result_tx,
            tokio_util::sync::CancellationToken::new(),
        ),
    )
    .await
    .expect("closed tool channel must stop forwarder");
    assert!(result_rx.recv().await.is_none());
}
