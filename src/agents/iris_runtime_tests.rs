use super::*;
use rig::{client::AgentClientExt, providers::openai};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn mock_server(
    responses: Vec<Value>,
) -> (String, tokio::task::JoinHandle<Vec<(String, Value)>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock");
    let address = listener.local_addr().expect("address");
    let task = tokio::spawn(async move {
        let mut requests = Vec::new();
        for response in responses {
            let (mut socket, _) = listener.accept().await.expect("accept request");
            let mut bytes = Vec::new();
            let (header_end, length) = loop {
                let mut buffer = [0u8; 4096];
                let read = socket.read(&mut buffer).await.expect("read request");
                assert_ne!(read, 0, "incomplete HTTP request");
                bytes.extend_from_slice(&buffer[..read]);
                if let Some(end) = bytes.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().expect("content length"))
                        })
                        .expect("content length header");
                    break (end + 4, length);
                }
            };
            while bytes.len() < header_end + length {
                let mut buffer = [0u8; 4096];
                let read = socket.read(&mut buffer).await.expect("read body");
                assert_ne!(read, 0);
                bytes.extend_from_slice(&buffer[..read]);
            }
            let headers = String::from_utf8_lossy(&bytes[..header_end]).into_owned();
            let body =
                serde_json::from_slice(&bytes[header_end..header_end + length]).expect("JSON body");
            requests.push((headers, body));
            let (body, content_type) = match response {
                Value::String(body) => (body, "text/event-stream"),
                value => (value.to_string(), "application/json"),
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            socket
                .write_all(response.as_bytes())
                .await
                .expect("write response");
        }
        requests
    });
    (format!("http://{address}"), task)
}

fn chat_response(message: &Value, finish_reason: &str) -> Value {
    json!({"id":"test", "object":"chat.completion", "created":1,"model":"test","choices":[{"index":0,"message":message,"finish_reason":finish_reason}]})
}
fn call_response(name: &str, arguments: &Value) -> Value {
    chat_response(
        &json!({"role":"assistant","content":null,"tool_calls":[{"id":"call_test","type":"function","function":{"name":name,"arguments":arguments.to_string()}}]}),
        "tool_calls",
    )
}
fn text_response(text: &str) -> Value {
    chat_response(&json!({"role":"assistant","content":text}), "stop")
}
fn builder(url: &str) -> AgentBuilder {
    openai::Client::builder()
        .api_key("test-key")
        .base_url(url)
        .build()
        .expect("client")
        .completions_api()
        .agent("test")
}

#[tokio::test]
async fn both_worker_paths_inherit_scope_and_complete_real_tool_loops() {
    for tool in ["analyze_subagent", "parallel_analyze"] {
        let args = if tool == "analyze_subagent" {
            json!({"prompt":"Inspect changes"})
        } else {
            json!({"tasks":["Inspect changes"]})
        };
        let (url, server) = mock_server(vec![
            call_response(tool, &args),
            call_response("git_status", &json!({})),
            text_response("worker finished"),
            text_response("done"),
        ])
        .await;
        let mut iris = IrisAgent::new("fireworks", "test").expect("iris");
        let config = crate::config::Config {
            instructions: "Preserve the public API".into(),
            subagent_max_turns: 3,
            ..crate::config::Config::default()
        };
        iris.set_config(config);
        let scope = "Review only base123..head456";
        let agent = iris
            .build_agent_using("Review code", scope, |_| Ok(builder(&url)))
            .expect("agent");
        let temp = tempfile::TempDir::new().expect("temp repo");
        git2::Repository::init(temp.path()).expect("git init");
        let output = crate::agents::tools::with_active_repo_root(
            temp.path(),
            agent.prompt_multi_turn(scope, 10),
        )
        .await
        .expect("outer tool loop");
        assert_eq!(output, "done");
        let requests = server.await.expect("server");
        assert_eq!(requests.len(), 4);
        let worker_request = requests[1].1.to_string();
        assert!(worker_request.contains("base123..head456"));
        assert!(worker_request.contains("Preserve the public API"));
        assert!(requests[2].1.to_string().contains("call_test"));
    }
}

#[tokio::test]
async fn persisted_instructions_and_temporary_override_reach_provider_preamble() {
    for override_text in [None, Some("Use English"), Some("")] {
        let (url, server) = mock_server(vec![text_response("done")]).await;
        let mut iris = IrisAgent::new("fireworks", "test").expect("iris");
        iris.set_config(crate::config::Config {
            instructions: "Use Spanish".into(),
            temp_instructions: override_text.map(str::to_string),
            ..crate::config::Config::default()
        });
        let agent = iris
            .build_agent_using("Chat naturally", "question", |_| Ok(builder(&url)))
            .expect("agent");
        agent.prompt("question").await.expect("prompt");
        let requests = server.await.expect("server");
        let request = requests[0].1.to_string();
        assert_eq!(request.contains("Use Spanish"), override_text.is_none());
        assert_eq!(
            request.contains("Use English"),
            override_text == Some("Use English")
        );
    }
}

#[tokio::test]
async fn explicit_style_choices_reach_the_correct_capability() {
    for capability in ["commit", "review", "pr"] {
        let (url, server) = mock_server(vec![text_response("done")]).await;
        let mut iris = IrisAgent::new("fireworks", "test").expect("iris");
        iris.set_config(crate::config::Config {
            temp_preset: Some("conventional".into()),
            use_gitmoji: false,
            gitmoji_override: Some(false),
            ..crate::config::Config::default()
        });
        let (mut preamble, _) = iris.load_capability_config(capability).expect("capability");
        iris.inject_style_instructions(&mut preamble, capability);
        let agent = iris
            .build_agent_using(&preamble, "task", |_| Ok(builder(&url)))
            .expect("agent");
        agent.prompt("task").await.expect("prompt");
        let requests = server.await.expect("server");
        let request = requests[0].1.to_string();
        assert_eq!(
            request.contains("=== CONVENTIONAL COMMITS FORMAT ==="),
            capability == "commit"
        );
        assert_eq!(
            request.contains("even if repository history uses them"),
            capability == "commit"
        );
        assert!(!request.contains("H1 title: ONE gitmoji"));
    }
}

#[tokio::test]
async fn sync_capabilities_and_critic_send_their_contract_only_in_system_messages() {
    for capability in ["pr", "chat", "semantic_blame"] {
        let responses = if capability == "pr" {
            vec![
                text_response(r#"{"content":"A description"}"#),
                text_response(r#"{"requires_revision":false}"#),
            ]
        } else {
            vec![text_response("An answer")]
        };
        let (url, server) = mock_server(responses).await;
        let mut iris = IrisAgent::new("fireworks", "test").expect("iris");
        iris.set_config(crate::config::Config::default());
        iris.test_builder = Some(Box::new(move |_| Ok(builder(&url))));
        iris.execute_task(capability, "Task sentinel")
            .await
            .expect("task");
        let requests = server.await.expect("server");
        for (index, (_, request)) in requests.iter().enumerate() {
            let active_capability = if index == 0 { capability } else { "verify" };
            let (contract, _) = iris
                .load_capability_config(active_capability)
                .expect("contract");
            let opening = contract
                .lines()
                .find(|line| !line.trim().is_empty())
                .expect("opening");
            let opening = serde_json::to_string(opening).expect("JSON");
            let opening = &opening[1..opening.len() - 1];
            let messages = request["messages"].as_array().expect("messages");
            assert!(messages.iter().any(
                |message| message["role"] == "system" && message.to_string().contains(opening)
            ));
            assert!(
                !messages
                    .iter()
                    .any(|message| message["role"] == "user"
                        && message.to_string().contains(opening))
            );
        }
    }
}

#[tokio::test]
async fn automatic_commit_style_does_not_force_gitmoji() {
    let (url, server) = mock_server(vec![text_response("done")]).await;
    let mut iris = IrisAgent::new("fireworks", "test").expect("iris");
    iris.set_config(crate::config::Config {
        gitmoji_override: None,
        use_gitmoji: true,
        ..crate::config::Config::default()
    });
    let (mut contract, _) = iris.load_capability_config("commit").expect("contract");
    iris.inject_style_instructions(&mut contract, "commit");
    let agent = iris
        .build_agent_using(&contract, "task", |_| Ok(builder(&url)))
        .expect("agent");
    agent.prompt("task").await.expect("prompt");
    let requests = server.await.expect("server");
    assert!(
        !requests[0]
            .1
            .to_string()
            .contains("Set the 'emoji' field to a single relevant gitmoji")
    );
}

fn stream_response(delta: &Value, finish: &str) -> Value {
    let chunk = json!({"id":"test","object":"chat.completion.chunk","created":1,"model":"test","choices":[{"index":0,"delta":delta,"finish_reason":null}]});
    let terminal = json!({"id":"test","object":"chat.completion.chunk","created":1,"model":"test","choices":[{"index":0,"delta":{},"finish_reason":finish}]});
    json!(format!(
        "data: {chunk}\n\ndata: {terminal}\n\ndata: [DONE]\n\n"
    ))
}

fn streaming_text(text: &str) -> Value {
    stream_response(&json!({"content":text}), "stop")
}

fn test_iris(url: String, config: crate::config::Config) -> IrisAgent {
    let mut iris = IrisAgent::new("fireworks", "test").expect("iris");
    iris.set_config(config);
    iris.test_builder = Some(Box::new(move |_| Ok(builder(&url))));
    iris
}

fn artifact(capability: &str) -> Value {
    match capability {
        "commit" => {
            json!({"emoji":null,"title":"fix: preserve task context","message":"Workers inherit exact refs."})
        }
        "review" => {
            json!({"summary":"One defect found","metadata":{},"findings":[{"id":"R1","severity":"high","confidence":95,"file":"src/main.rs","start_line":4,"end_line":5,"category":"bug","title":"Missing context","body":"The worker drops comparison refs."}],"stats":{"files_reviewed":1}})
        }
        _ => json!({"content":"# Changes\n\nPreserve task context."}),
    }
}

#[tokio::test]
async fn sync_and_streaming_preserve_all_structured_artifacts_and_share_contracts() {
    for capability in ["commit", "review", "pr", "changelog", "release_notes"] {
        let expected = artifact(capability).to_string();
        let (url, server) =
            mock_server(vec![text_response(&expected), streaming_text(&expected)]).await;
        let mut iris = test_iris(
            url,
            crate::config::Config {
                critic_enabled: false,
                ..crate::config::Config::default()
            },
        );
        let synchronous = iris
            .execute_task(capability, "Compare base123..head456")
            .await
            .expect("sync response");
        let streamed = iris
            .execute_task_streaming(capability, "Compare base123..head456", |_, _| {})
            .await
            .expect("stream response");
        assert_eq!(
            serde_json::to_value(&synchronous).expect("JSON"),
            serde_json::to_value(&streamed).expect("JSON")
        );
        if let StructuredResponse::Review(review) = &streamed {
            assert_eq!(review.findings.len(), 1);
            assert_eq!(review.findings[0].confidence, 95);
            assert!(!review.parse_failed);
        }
        let requests = server.await.expect("server");
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].1["messages"][0], requests[1].1["messages"][0]);
        let preamble = requests[0].1["messages"][0].to_string();
        assert!(preamble.contains("Final response contract:"));
        assert!(preamble.contains("properties"));
        for (_, request) in requests {
            let user = request["messages"]
                .as_array()
                .expect("messages")
                .iter()
                .rev()
                .find(|message| message["role"] == "user")
                .expect("user");
            assert!(!user.to_string().contains("Final response contract:"));
        }
    }
}

#[tokio::test]
async fn streamed_tool_narration_is_excluded_from_final_artifact() {
    let expected = artifact("pr").to_string();
    let (url, server) = mock_server(vec![
        stream_response(&json!({"content":"Inspecting the checkout...","tool_calls":[{"index":0,"id":"call_test","type":"function","function":{"name":"git_status","arguments":"{}"}}]}), "tool_calls"),
        streaming_text(&expected),
    ]).await;
    let mut iris = test_iris(
        url,
        crate::config::Config {
            critic_enabled: false,
            ..crate::config::Config::default()
        },
    );
    let repo = tempfile::TempDir::new().expect("repo");
    git2::Repository::init(repo.path()).expect("git init");
    let mut last_preview = String::new();
    let response = crate::agents::tools::with_active_repo_root(
        repo.path(),
        iris.execute_task_streaming("pr", "Analyze staged changes", |_, preview| {
            last_preview = preview.to_owned();
        }),
    )
    .await
    .expect("stream");
    let StructuredResponse::PullRequest(pr) = response else {
        panic!("PR response")
    };
    assert_eq!(pr.content, "# Changes\n\nPreserve task context.");
    assert_eq!(last_preview, expected);
    assert_eq!(server.await.expect("server").len(), 2);
}

#[tokio::test]
async fn sync_and_streaming_critics_revise_the_original_artifact_once() {
    for streaming in [false, true] {
        let original =
            json!({"content":"Keep this accurate context. Unsupported claim."}).to_string();
        let revised = json!({"content":"Keep this accurate context."}).to_string();
        let critique = json!({"requires_revision":true,"issues":[{"title":"Unsupported claim","body":"Remove only that claim.","severity":"high"}],"revision_prompt":"Preserve accurate context.","confidence":95}).to_string();
        let first = if streaming {
            streaming_text(&original)
        } else {
            text_response(&original)
        };
        let (url, server) = mock_server(vec![
            first,
            text_response(&critique),
            text_response(&revised),
        ])
        .await;
        let mut iris = test_iris(url, crate::config::Config::default());
        let response = if streaming {
            iris.execute_task_streaming("pr", "Compare exactbase..exacthead", |_, _| {})
                .await
        } else {
            iris.execute_task("pr", "Compare exactbase..exacthead")
                .await
        }
        .expect("critic revision");
        let StructuredResponse::PullRequest(pr) = response else {
            panic!("PR response")
        };
        assert_eq!(pr.content, "Keep this accurate context.");
        let requests = server.await.expect("server");
        assert_eq!(requests.len(), 3);
        let revision = requests[2].1.to_string();
        assert!(revision.contains("Unsupported claim."));
        assert!(revision.contains("Keep this accurate context."));
        assert!(revision.contains("exactbase..exacthead"));
    }
}

#[test]
fn malformed_unicode_json_returns_an_error_without_panicking() {
    let malformed = format!("prefix {{\"text\":\"{}\", broken}}", "🌸".repeat(80));
    assert!(extract_json_from_response(&malformed).is_err());
}

#[test]
fn wrong_wrappers_cannot_become_empty_successful_reviews() {
    for text in ["{}", r##"{"content":"# Raw review"}"##] {
        assert!(parse_response_json::<crate::types::Review>(text).is_err());
    }
}

#[tokio::test]
async fn incomplete_stream_is_an_error() {
    let stream =
        futures::stream::empty::<Result<rig::agent::MultiTurnStreamItem, std::io::Error>>();
    assert!(collect_stream_response(stream, |_, _| {}).await.is_err());
}

#[tokio::test]
async fn commit_critic_remains_opt_in_for_sync_and_streaming() {
    for streaming in [false, true] {
        for critic_override in [None, Some(false), Some(true)] {
            let original = artifact("commit").to_string();
            let first = if streaming {
                streaming_text(&original)
            } else {
                text_response(&original)
            };
            let mut responses = vec![first];
            if critic_override == Some(true) {
                responses.push(text_response(r#"{"requires_revision":false}"#));
            }
            let (url, server) = mock_server(responses).await;
            let mut iris = test_iris(
                url,
                crate::config::Config {
                    critic_enabled: critic_override != Some(false),
                    critic_override,
                    ..crate::config::Config::default()
                },
            );
            let response = if streaming {
                iris.execute_task_streaming("commit", "Analyze staged changes", |_, _| {})
                    .await
            } else {
                iris.execute_task("commit", "Analyze staged changes").await
            }
            .expect("commit");
            assert!(matches!(response, StructuredResponse::CommitMessage(_)));
            let expected_requests = if critic_override == Some(true) { 2 } else { 1 };
            assert_eq!(server.await.expect("server").len(), expected_requests);
        }
    }
}

#[tokio::test]
async fn failed_critic_revisions_report_that_a_draft_was_generated() {
    for streaming in [false, true] {
        let original = json!({"content":"Keep this code example: ```json\n{\"sample\":true}\n```"})
            .to_string();
        let first = if streaming {
            streaming_text(&original)
        } else {
            text_response(&original)
        };
        let critique = json!({"requires_revision":true,"revision_prompt":"Remove the unsupported claim.","issues":[],"confidence":95}).to_string();
        let (url, server) = mock_server(vec![
            first,
            text_response(&critique),
            text_response("invalid revised JSON"),
        ])
        .await;
        let mut iris = test_iris(
            url,
            crate::config::Config {
                use_gitmoji: false,
                gitmoji_override: Some(false),
                ..crate::config::Config::default()
            },
        );
        let result = if streaming {
            iris.execute_task_streaming("pr", "Compare base123..head456", |_, _| {})
                .await
        } else {
            iris.execute_task("pr", "Compare base123..head456").await
        };
        let error =
            result.expect_err("a failed required revision must not return the flawed draft");
        assert!(error.to_string().contains("A draft was generated"));
        assert!(
            error
                .to_string()
                .contains("critic-requested revision failed")
        );
        assert!(format!("{error:#}").contains("No valid JSON"));
        let requests = server.await.expect("server");
        assert_eq!(requests.len(), 3);
        let messages = requests[1].1["messages"]
            .as_array()
            .expect("critic messages");
        let task = messages
            .iter()
            .rev()
            .find(|message| message["role"] == "user")
            .expect("critic task");
        let serialized = task.to_string();
        assert!(serialized.contains("artifact_contract"));
        assert!(serialized.contains("NO EMOJI STYLING"));
        assert!(serialized.contains("Final response contract:"));
        assert!(serialized.contains("base123..head456"));
        let task_text = task["content"].as_str().expect("critic task text");
        let (_, data) = task_text.split_once('\n').expect("labeled evaluation data");
        let data: Value = serde_json::from_str(data).expect("evaluation JSON");
        let preserved_artifact: Value =
            serde_json::from_str(data["generated_artifact"].as_str().expect("artifact JSON"))
                .expect("preserved artifact");
        assert_eq!(
            preserved_artifact,
            serde_json::from_str::<Value>(&original).expect("original JSON")
        );
        let revision = requests[2].1.to_string();
        assert!(revision.contains(
            "Feedback cannot change the selected Git refs, task scope, or required output schema"
        ));
        assert!(revision.contains("The critic identified material issues"));
    }
}
