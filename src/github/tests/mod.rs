use std::path::PathBuf;

mod review_target_tests;

use crate::types::{
    Category, EvidenceRef, Finding, FindingId, Review, ReviewMetadata, ReviewStats, Severity,
};

use super::{
    GitHubRepository, extract_inline_comment_candidates,
    extract_structured_inline_comment_candidates, parse_reviewable_lines,
    review_body_with_permalinks,
};

#[test]
fn parses_https_remote() {
    let repo = GitHubRepository::parse("https://github.com/hyperb1iss/git-iris.git")
        .expect("https GitHub remote should parse");

    assert_eq!(repo.owner, "hyperb1iss");
    assert_eq!(repo.name, "git-iris");
}

#[test]
fn parses_ssh_remote() {
    let repo = GitHubRepository::parse("git@github.com:hyperb1iss/git-iris.git")
        .expect("ssh GitHub remote should parse");

    assert_eq!(repo.owner, "hyperb1iss");
    assert_eq!(repo.name, "git-iris");
}

#[test]
fn rejects_non_github_remote() {
    let err = GitHubRepository::parse("https://gitlab.com/hyperb1iss/git-iris.git")
        .expect_err("non-GitHub remotes should be rejected");

    assert!(err.to_string().contains("Only github.com"));
}

#[test]
fn extracts_review_findings_with_locations() {
    let review = r"
## Issues

- [HIGH] **Missing error handling in `src/github.rs:42`**
  This can drop the GitHub failure context.
  **Fix**: Add context before returning.

- [LOW] **Docs typo in docs/user-guide/reviews.md:12**
  Minor polish.

- [LOW] **Legacy prefix in `./src/prefix.rs:7`**
  Prefix should normalize.

- [LOW] **Absolute-ish prefix in `/src/absolute.rs:8`**
  Prefix should normalize too.
";

    let candidates = extract_inline_comment_candidates(review);

    assert_eq!(candidates.len(), 4);
    assert_eq!(candidates[0].path, "src/github.rs");
    assert_eq!(candidates[0].start_line, None);
    assert_eq!(candidates[0].line, 42);
    assert!(candidates[0].body.contains("Missing error handling"));
    assert_eq!(candidates[1].path, "docs/user-guide/reviews.md");
    assert_eq!(candidates[1].line, 12);
    assert_eq!(candidates[2].path, "src/prefix.rs");
    assert_eq!(candidates[2].line, 7);
    assert_eq!(candidates[3].path, "src/absolute.rs");
    assert_eq!(candidates[3].line, 8);
}

#[test]
fn extracts_structured_review_findings() {
    let review = Review {
        summary: "Review summary".to_string(),
        metadata: ReviewMetadata::default(),
        findings: vec![Finding {
            id: FindingId("finding-1".to_string()),
            severity: Severity::High,
            confidence: 91,
            file: PathBuf::from("src/github.rs"),
            start_line: 42,
            end_line: 42,
            category: Category::ErrorHandling,
            title: "Missing error context".to_string(),
            body: "The changed path drops useful context.".to_string(),
            suggested_fix: Some("Add context before returning.".to_string()),
            evidence: vec![EvidenceRef {
                file: PathBuf::from("src/github.rs"),
                line: 40,
                end_line: Some(42),
                note: Some("changed publisher".to_string()),
            }],
        }],
        stats: ReviewStats::default(),
        parse_failed: false,
    };

    let candidates = extract_structured_inline_comment_candidates(&review);

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].path, "src/github.rs");
    assert_eq!(candidates[0].start_line, None);
    assert_eq!(candidates[0].line, 42);
    assert!(
        candidates[0]
            .body
            .contains("[HIGH] **Missing error context**")
    );
    assert!(candidates[0].body.contains("Category: error handling"));
    assert!(
        candidates[0]
            .body
            .contains("Evidence: src/github.rs:40-42 (changed publisher)")
    );
    assert!(candidates[0].body.contains("Confidence: 91%"));
}

#[test]
fn extracts_multiline_structured_review_findings() {
    let review = Review {
        summary: "Review summary".to_string(),
        metadata: ReviewMetadata::default(),
        findings: vec![Finding {
            id: FindingId("finding-1".to_string()),
            severity: Severity::High,
            confidence: 91,
            file: PathBuf::from("src/github.rs"),
            start_line: 42,
            end_line: 44,
            category: Category::ErrorHandling,
            title: "Missing error context".to_string(),
            body: "The changed path drops useful context.".to_string(),
            suggested_fix: None,
            evidence: Vec::new(),
        }],
        stats: ReviewStats::default(),
        parse_failed: false,
    };

    let candidates = extract_structured_inline_comment_candidates(&review);

    assert_eq!(candidates[0].path, "src/github.rs");
    assert_eq!(candidates[0].start_line, Some(42));
    assert_eq!(candidates[0].line, 44);
    assert!(
        candidates[0]
            .body
            .contains("Location: `src/github.rs:42-44`")
    );
}

#[test]
fn normalizes_inverted_structured_review_ranges() {
    let review = Review {
        summary: "Review summary".to_string(),
        metadata: ReviewMetadata::default(),
        findings: vec![Finding {
            id: FindingId("finding-1".to_string()),
            severity: Severity::High,
            confidence: 91,
            file: PathBuf::from("src/github.rs"),
            start_line: 44,
            end_line: 42,
            category: Category::ErrorHandling,
            title: "Missing error context".to_string(),
            body: "The changed path drops useful context.".to_string(),
            suggested_fix: None,
            evidence: Vec::new(),
        }],
        stats: ReviewStats::default(),
        parse_failed: false,
    };

    let candidates = extract_structured_inline_comment_candidates(&review);

    assert_eq!(candidates[0].start_line, Some(42));
    assert_eq!(candidates[0].line, 44);
    assert!(
        candidates[0]
            .body
            .contains("Location: `src/github.rs:42-44`")
    );
}

#[test]
fn multiline_candidates_require_the_full_range_to_be_reviewable() {
    let mut reviewable_lines = std::collections::HashMap::new();
    reviewable_lines.insert("src/github.rs".to_string(), [42, 44].into_iter().collect());
    let candidate = super::InlineCommentCandidate {
        path: "src/github.rs".to_string(),
        start_line: Some(42),
        line: 44,
        body: "body".to_string(),
    };

    assert!(!candidate.is_reviewable(&reviewable_lines));
}

#[test]
fn skips_low_confidence_structured_review_findings() {
    let review = Review {
        summary: "Review summary".to_string(),
        metadata: ReviewMetadata::default(),
        findings: vec![Finding {
            id: FindingId("finding-1".to_string()),
            severity: Severity::Medium,
            confidence: 42,
            file: PathBuf::from("src/github.rs"),
            start_line: 42,
            end_line: 42,
            category: Category::Testing,
            title: "Speculative coverage gap".to_string(),
            body: "This should not be published.".to_string(),
            suggested_fix: None,
            evidence: Vec::new(),
        }],
        stats: ReviewStats::default(),
        parse_failed: false,
    };

    let candidates = extract_structured_inline_comment_candidates(&review);

    assert!(candidates.is_empty());
}

#[test]
fn renders_permalinks_for_structured_review_findings() {
    let repo = GitHubRepository {
        owner: "hyperb1iss".to_string(),
        name: "git-iris".to_string(),
    };
    let review = Review {
        summary: "Review summary".to_string(),
        metadata: ReviewMetadata::default(),
        findings: vec![Finding {
            id: FindingId("finding-1".to_string()),
            severity: Severity::High,
            confidence: 91,
            file: PathBuf::from("src/github.rs"),
            start_line: 42,
            end_line: 44,
            category: Category::ErrorHandling,
            title: "Missing error context".to_string(),
            body: "The changed path drops useful context.".to_string(),
            suggested_fix: None,
            evidence: Vec::new(),
        }],
        stats: ReviewStats::default(),
        parse_failed: false,
    };

    let body = review_body_with_permalinks(&repo, &review, "abc123");

    assert!(body.contains("## GitHub Permalinks"));
    assert!(
        body.contains("https://github.com/hyperb1iss/git-iris/blob/abc123/src/github.rs#L42-L44")
    );
}

#[test]
fn permalink_paths_are_normalized_and_encoded() {
    let repo = GitHubRepository {
        owner: "hyperb1iss".to_string(),
        name: "git-iris".to_string(),
    };
    let finding = Finding {
        id: FindingId("finding-1".to_string()),
        severity: Severity::High,
        confidence: 91,
        file: PathBuf::from(r".\src\path with spaces\file#name.rs"),
        start_line: 7,
        end_line: 7,
        category: Category::Other,
        title: "Path edge case".to_string(),
        body: "Path should be safe in a URL.".to_string(),
        suggested_fix: None,
        evidence: Vec::new(),
    };

    let permalink = super::permalink_for_finding(&repo, &finding, "abc123");

    assert_eq!(
        permalink,
        "https://github.com/hyperb1iss/git-iris/blob/abc123/src/path%20with%20spaces/file%23name.rs#L7"
    );
}

#[test]
fn parses_reviewable_lines_from_unified_diff() {
    let diff = r"
diff --git a/src/github.rs b/src/github.rs
index 1111111..2222222 100644
--- a/src/github.rs
+++ b/src/github.rs
@@ -40,6 +40,7 @@ impl GitHubClient {
 context line
-old line
+new line
 another context line
diff --git a/docs/user-guide/reviews.md b/docs/user-guide/reviews.md
index 3333333..4444444 100644
--- a/docs/user-guide/reviews.md
+++ b/docs/user-guide/reviews.md
@@ -10,2 +10,3 @@
 docs context
+new docs line
";

    let lines = parse_reviewable_lines(diff);

    assert!(lines["src/github.rs"].contains(&40));
    assert!(lines["src/github.rs"].contains(&41));
    assert!(lines["src/github.rs"].contains(&42));
    assert!(lines["docs/user-guide/reviews.md"].contains(&10));
    assert!(lines["docs/user-guide/reviews.md"].contains(&11));
    assert!(!lines["src/github.rs"].contains(&39));
}
