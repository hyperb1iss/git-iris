#![allow(clippy::unwrap_used)]

use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct Capability {
    name: String,
    description: String,
    output_type: String,
    task_prompt: String,
}

#[test]
fn embedded_capabilities_parse_with_their_runtime_output_contracts() {
    let contracts = [
        ("commit", "GeneratedMessage"),
        ("review", "Review"),
        ("pr", "MarkdownPullRequest"),
        ("changelog", "MarkdownChangelog"),
        ("release_notes", "MarkdownReleaseNotes"),
        ("chat", "PlainText"),
        ("semantic_blame", "SemanticBlame"),
        ("verify", "Critique"),
    ];
    for (name, output_type) in contracts {
        let text = fs::read_to_string(format!("src/agents/capabilities/{name}.toml")).unwrap();
        let capability: Capability = toml::from_str(&text).unwrap();
        assert_eq!(capability.name, name);
        assert_eq!(capability.output_type, output_type);
        assert!(!capability.description.trim().is_empty());
        assert!(!capability.task_prompt.trim().is_empty());
    }
}
