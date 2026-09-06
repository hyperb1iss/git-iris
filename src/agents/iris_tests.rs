use super::{
    Critique, CritiqueIssue, CritiqueSeverity, IrisAgent, extract_json_from_response,
    find_balanced_braces, sanitize_json_response,
};
use serde_json::Value;
use std::borrow::Cow;

#[test]
fn sanitize_json_response_is_noop_for_valid_payloads() {
    let raw = r#"{"title":"Test","description":"All good"}"#;
    let sanitized = sanitize_json_response(raw);
    assert!(matches!(sanitized, Cow::Borrowed(_)));
    serde_json::from_str::<Value>(sanitized.as_ref()).expect("valid JSON");
}

#[test]
fn sanitize_json_response_escapes_literal_newlines() {
    let raw = "{\"description\": \"Line1
Line2\"}";
    let sanitized = sanitize_json_response(raw);
    assert_eq!(sanitized.as_ref(), "{\"description\": \"Line1\\nLine2\"}");
    serde_json::from_str::<Value>(sanitized.as_ref()).expect("json sanitized");
}

#[test]
fn find_balanced_braces_returns_first_balanced_pair() {
    let (start, end) = find_balanced_braces("prefix {\"a\":1} suffix").expect("balanced pair");
    assert_eq!(&"prefix {\"a\":1} suffix"[start..end], "{\"a\":1}");
}

#[test]
fn find_balanced_braces_returns_none_for_unbalanced() {
    assert_eq!(find_balanced_braces("no braces here"), None);
    assert_eq!(find_balanced_braces("{ unclosed"), None);
}

#[test]
fn extract_json_skips_github_actions_expression_false_positive() {
    // Regression for a real failure: a diff hunk that adds
    // `commit_message: "Update to ${{ github.ref_name }}"` to a workflow
    // lands in the model's response. The old scanner grabbed `{{ github.ref_name }}`
    // as its first balanced pair and errored out before seeing the real JSON.
    let response = r#"Looking at the diff, I see the new value `${{ github.ref_name }}` replacing the old bash expansion. Here's the commit:

{"emoji": "🔧", "title": "Upgrade AUR deploy action", "message": "Bump to v4.1.2 to fix bash --command error."}
"#;
    let extracted = extract_json_from_response(response).expect("should recover real JSON");
    let parsed: Value = serde_json::from_str(&extracted).expect("extracted value is JSON");
    assert_eq!(parsed["emoji"], "🔧");
    assert_eq!(parsed["title"], "Upgrade AUR deploy action");
}

#[test]
fn extract_json_from_pure_json_response() {
    let response = r##"{"content": "# Heading\n\nBody text."}"##;
    let extracted = extract_json_from_response(response).expect("pure JSON passes through");
    assert_eq!(extracted, response);
}

#[test]
fn streamed_generated_message_text_becomes_commit_response() {
    let response = r#"```json
{"emoji":"🔧","title":"Wire streaming commit output","message":"Parse streamed JSON into the commit response type."}
```"#;

    let structured =
        IrisAgent::text_to_structured_response("GeneratedMessage", response.to_string())
            .expect("commit response");

    let super::StructuredResponse::CommitMessage(message) = structured else {
        panic!("expected commit message response");
    };
    assert_eq!(message.emoji.as_deref(), Some("🔧"));
    assert_eq!(message.title, "Wire streaming commit output");
    assert_eq!(
        message.message,
        "Parse streamed JSON into the commit response type."
    );
}

#[test]
fn invalid_streamed_generated_message_returns_error() {
    assert!(
        IrisAgent::text_to_structured_response("GeneratedMessage", "not json".to_string()).is_err()
    );
}

#[test]
fn critic_runs_for_configured_structured_artifacts() {
    let mut agent = IrisAgent::new("openai", "gpt-5.4").expect("agent should build");
    agent.set_config(crate::config::Config::default());

    assert!(agent.should_run_critic("review", "Review"));
    assert!(!agent.should_run_critic("commit", "GeneratedMessage"));
    assert!(!agent.should_run_critic("chat", "PlainText"));
    assert!(!agent.should_run_critic("semantic_blame", "SemanticBlame"));
}

#[test]
fn critic_runs_for_commits_when_explicitly_enabled() {
    let config = crate::config::Config {
        critic_override: Some(true),
        ..crate::config::Config::default()
    };
    let mut agent = IrisAgent::new("openai", "gpt-5.4").expect("agent should build");
    agent.set_config(config);

    assert!(agent.should_run_critic("commit", "GeneratedMessage"));
}

#[test]
fn critic_can_be_disabled_by_config() {
    let config = crate::config::Config {
        critic_enabled: false,
        ..crate::config::Config::default()
    };
    let mut agent = IrisAgent::new("openai", "gpt-5.4").expect("agent should build");
    agent.set_config(config);

    assert!(!agent.should_run_critic("review", "Review"));
}

#[test]
fn critic_revision_prompt_includes_material_issues() {
    let critique = Critique {
        requires_revision: true,
        issues: vec![CritiqueIssue {
            title: "Unsupported auth claim".to_string(),
            body: "The diff only updates docs.".to_string(),
            severity: CritiqueSeverity::High,
        }],
        revision_prompt: "Remove the auth-hardening claim.".to_string(),
        confidence: 91,
    };

    let prompt = IrisAgent::build_revision_prompt(
        "Original task",
        &super::StructuredResponse::PlainText("Original artifact".into()),
        &critique,
    );

    assert!(prompt.contains("Original task"));
    assert!(prompt.contains("[high] Unsupported auth claim"));
    assert!(prompt.contains("Remove the auth-hardening claim."));
    assert!(prompt.contains("private revision guidance"));
    assert!(prompt.contains("Do not mention the critic"));
}

#[test]
fn critic_revision_prompt_falls_back_to_issues() {
    let critique = Critique {
        requires_revision: true,
        issues: vec![CritiqueIssue {
            title: "Unsupported auth claim".to_string(),
            body: "The diff only updates docs.".to_string(),
            severity: CritiqueSeverity::High,
        }],
        revision_prompt: String::new(),
        confidence: 91,
    };

    let prompt = IrisAgent::build_revision_prompt(
        "Original task",
        &super::StructuredResponse::PlainText("Original artifact".into()),
        &critique,
    );

    assert!(prompt.contains("Address the material issues listed above."));
}

#[test]
fn critic_revision_prompt_omits_empty_issues_section() {
    let critique = Critique {
        requires_revision: true,
        issues: Vec::new(),
        revision_prompt: "Remove the unsupported claim.".to_string(),
        confidence: 91,
    };

    let prompt = IrisAgent::build_revision_prompt(
        "Original task",
        &super::StructuredResponse::PlainText("Original artifact".into()),
        &critique,
    );

    assert!(!prompt.contains("Issues:"));
    assert!(prompt.contains("Remove the unsupported claim."));
}

#[test]
fn critic_artifact_serialization_strips_response_variant_wrapper() {
    let response = super::StructuredResponse::CommitMessage(crate::types::GeneratedMessage {
        emoji: None,
        title: "Add critic pass".to_string(),
        message: "Check generated artifacts before returning them.".to_string(),
        completion_message: None,
    });

    let artifact = IrisAgent::serialize_artifact_for_critic(&response);

    assert!(artifact.contains("\"title\": \"Add critic pass\""));
    assert!(!artifact.contains("CommitMessage"));
}

#[test]
fn critic_severity_normalizes_unknown_values_to_medium() {
    let severity: CritiqueSeverity =
        serde_json::from_str("\"totally-fine\"").expect("severity should deserialize");

    assert_eq!(severity, CritiqueSeverity::Medium);
}

#[test]
fn extract_json_errors_when_no_candidate_parses() {
    // A single malformed candidate and no other braces: we surface the
    // parse error with a preview so the user sees what went wrong.
    let response = "prose ${{ template }} more prose";
    let err = extract_json_from_response(response).expect_err("should fail");
    let msg = err.to_string();
    assert!(
        msg.contains("Preview:"),
        "error should include a preview: {msg}"
    );
}

#[test]
fn pr_review_emoji_styling_uses_a_compact_gitmoji_guide() {
    let mut prompt = String::new();
    IrisAgent::inject_pr_review_emoji_styling(&mut prompt);

    assert!(prompt.contains("Common gitmoji choices:"));
    assert!(prompt.contains("`:feat:`"));
    assert!(prompt.contains("`:fix:`"));
    assert!(!prompt.contains("`:accessibility:`"));
    assert!(!prompt.contains("`:analytics:`"));
}
