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
            let body = response.to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
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
