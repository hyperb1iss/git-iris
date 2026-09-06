use crate::{agents::TaskContext, git::GitRepo, github::ReviewTarget};
use anyhow::Result;

#[test]
fn review_target_rejects_retargeting_or_changed_head() {
    let target = ReviewTarget {
        base_ref: "main".into(),
        base_sha: "base".into(),
        head_sha: "head".into(),
    };
    assert!(target.validate("main", "head").is_ok());
    assert!(target.validate("release", "head").is_err());
    assert!(target.validate("main", "new-head").is_err());
}

#[test]
fn github_review_context_pins_commits_and_rejects_unpublished_changes() -> Result<()> {
    let dir = tempfile::TempDir::new()?;
    let repo = git2::Repository::init(dir.path())?;
    let sig = git2::Signature::now("Test", "test@example.com")?;
    let tree_id = repo.index()?.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let base = repo.commit(Some("HEAD"), &sig, &sig, "base", &tree, &[])?;
    let parent = repo.find_commit(base)?;
    let head = repo.commit(Some("HEAD"), &sig, &sig, "head", &tree, &[&parent])?;
    let target = ReviewTarget {
        base_ref: "main".into(),
        base_sha: base.to_string(),
        head_sha: head.to_string(),
    };
    let git_repo = GitRepo::new(dir.path())?;

    let context = target.pin_context(
        &git_repo,
        TaskContext::Staged {
            include_unstaged: false,
        },
    )?;
    assert!(
        matches!(context, TaskContext::Range { from, to } if from == base.to_string() && to == head.to_string())
    );
    let context = target.pin_context(
        &git_repo,
        TaskContext::Commit {
            commit_id: "HEAD".into(),
        },
    )?;
    assert!(matches!(context, TaskContext::Commit { commit_id } if commit_id == head.to_string()));
    assert!(
        target
            .pin_context(
                &git_repo,
                TaskContext::Commit {
                    commit_id: base.to_string()
                }
            )
            .is_err()
    );
    assert!(
        target
            .pin_context(
                &git_repo,
                TaskContext::Range {
                    from: base.to_string(),
                    to: base.to_string()
                }
            )
            .is_err()
    );
    assert!(
        target
            .pin_context(
                &git_repo,
                TaskContext::Staged {
                    include_unstaged: true
                }
            )
            .is_err()
    );
    Ok(())
}

async fn server(
    responses: Vec<serde_json::Value>,
) -> Result<(
    crate::github::GitHubClient,
    tokio::task::JoinHandle<Vec<(String, serde_json::Value)>>,
)> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let task = tokio::spawn(async move {
        let mut requests = Vec::new();
        for response in responses {
            let (mut stream, _) = listener.accept().await.expect("accept");
            let mut bytes = Vec::new();
            let (end, length) = loop {
                let mut chunk = [0; 4096];
                let count = stream.read(&mut chunk).await.expect("read");
                assert_ne!(count, 0);
                bytes.extend_from_slice(&chunk[..count]);
                if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().expect("length"))
                        })
                        .unwrap_or(0);
                    break (end + 4, length);
                }
            };
            while bytes.len() < end + length {
                let mut chunk = [0; 4096];
                let count = stream.read(&mut chunk).await.expect("body");
                assert_ne!(count, 0);
                bytes.extend_from_slice(&chunk[..count]);
            }
            let header = String::from_utf8_lossy(&bytes[..end]).into_owned();
            let body = if length == 0 {
                serde_json::Value::Null
            } else {
                serde_json::from_slice(&bytes[end..end + length]).expect("JSON body")
            };
            requests.push((header, body));
            let body = response.to_string();
            stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.expect("response");
        }
        requests
    });
    let client = crate::github::GitHubClient {
        crab: octocrab::Octocrab::builder()
            .base_uri(format!("http://{address}"))?
            .build()?,
        repo: crate::github::GitHubRepository {
            owner: "test".into(),
            name: "repo".into(),
        },
    };
    Ok((client, task))
}

fn pull(head: &str) -> serde_json::Value {
    serde_json::json!({"id":1,"number":1,"url":"https://api.github.com/repos/test/repo/pulls/1",
        "head":{"ref":"feature","sha":head},"base":{"ref":"main","sha":"base"}})
}

#[tokio::test]
async fn publisher_posts_only_the_reviewed_commit() -> Result<()> {
    let mut advanced_base = pull("head");
    advanced_base["base"]["sha"] = "advanced-base".into();
    let (client, requests) = server(vec![
        pull("head"),
        advanced_base,
        serde_json::json!({
        "id":1,"node_id":"review","html_url":"https://github.com/test/repo/pull/1#review"}),
    ])
    .await?;
    let target = ReviewTarget {
        base_ref: "main".into(),
        base_sha: "base".into(),
        head_sha: "head".into(),
    };
    client
        .publish_review(
            1,
            "Reviewed pinned changes",
            crate::github::ReviewPublishOptions {
                event: octocrab::models::pulls::ReviewAction::Comment,
                inline_comments: false,
            },
            &target,
        )
        .await?;
    let requests = requests.await?;
    assert_eq!(requests.len(), 3);
    assert!(
        requests[2]
            .0
            .starts_with("POST /repos/test/repo/pulls/1/reviews ")
    );
    assert_eq!(requests[2].1["commit_id"], "head");
    Ok(())
}

#[tokio::test]
async fn publisher_rejects_head_movement_before_posting() -> Result<()> {
    let (client, requests) = server(vec![pull("head"), pull("new-head")]).await?;
    let target = ReviewTarget {
        base_ref: "main".into(),
        base_sha: "base".into(),
        head_sha: "head".into(),
    };
    let error = client
        .publish_review(
            1,
            "Review",
            crate::github::ReviewPublishOptions {
                event: octocrab::models::pulls::ReviewAction::Approve,
                inline_comments: false,
            },
            &target,
        )
        .await
        .expect_err("moving PR must not publish");
    assert!(error.to_string().contains("changed during analysis"));
    let requests = requests.await?;
    assert_eq!(requests.len(), 2);
    assert!(
        requests
            .iter()
            .all(|(header, _)| header.starts_with("GET "))
    );
    Ok(())
}
