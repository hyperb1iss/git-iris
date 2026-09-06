use super::super::*;
use rig::tool::portable::PortableTool;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct Probe;
impl PortableTool for Probe {
    const NAME: &'static str = "probe";
    type Error = std::io::Error;
    type Args = Value;
    type Output = String;

    fn description(&self) -> String {
        "Return a test result".to_string()
    }
    fn parameters(&self) -> Value {
        json!({"type":"object", "properties":{}, "additionalProperties":false})
    }
    fn call(&self, _args: Value) -> impl std::future::Future<Output = Result<String, Self::Error>> {
        std::future::ready(Ok("verified".to_string()))
    }
}

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

fn responses_result(output: &Value) -> Value {
    json!({"id":"resp_test", "object":"response", "created_at":1, "status":"completed", "model":"gpt-6-astra", "output":output})
}

fn chat_result(message: &Value, finish: &str) -> Value {
    json!({"id":"chat_test", "object":"chat.completion", "created":1, "model":"test", "choices":[{"index":0,"message":message,"finish_reason":finish}]})
}

#[tokio::test]
async fn responses_preserve_reasoning_and_tool_results_across_turns() {
    let first = responses_result(&json!([
        {"type":"reasoning","id":"rs_test","summary":[],"encrypted_content":"encrypted-test"},
        {"type":"function_call","id":"fc_test","call_id":"call_test","name":"probe","arguments":"{}","status":"completed"}
    ]));
    let last = responses_result(
        &json!([{"type":"message","id":"msg_test","role":"assistant","status":"completed","content":[{"type":"output_text","text":"done","annotations":[]}]}]),
    );
    let (url, server) = mock_server(vec![first, last]).await;
    let builder = agent_builder_at(
        Provider::OpenAI,
        "gpt-6-astra",
        Some("test-key"),
        Some(&url),
    )
    .expect("builder");
    let agent = apply_completion_params::<_, std::collections::hash_map::RandomState>(
        builder,
        Provider::OpenAI,
        "gpt-6-astra",
        4096,
        None,
        CompletionProfile::MainAgent,
    )
    .tool(Probe)
    .build();
    assert_eq!(
        agent.prompt("test").max_turns(3).await.expect("tool loop"),
        "done"
    );
    let requests = server.await.expect("mock task");
    assert_eq!(requests.len(), 2);
    for (headers, body) in &requests {
        assert!(headers.starts_with("POST /responses "));
        assert!(
            headers
                .to_lowercase()
                .contains("authorization: bearer test-key")
        );
        assert_eq!(body["reasoning"]["effort"], "medium");
        assert_eq!(body["max_output_tokens"], 4096);
        assert!(body.get("max_tokens").is_none());
        assert!(body.get("max_completion_tokens").is_none());
        assert!(body.get("temperature").is_none());
    }
    let input = requests[1].1["input"].as_array().expect("input");
    assert!(
        input.iter().any(
            |item| item["type"] == "reasoning" && item["encrypted_content"] == "encrypted-test"
        )
    );
    assert!(
        input
            .iter()
            .any(|item| item["type"] == "function_call_output" && item["call_id"] == "call_test")
    );
}

#[tokio::test]
async fn routed_chat_preserves_provider_reasoning_and_tool_results() {
    for provider in [Provider::OpenRouter, Provider::Fireworks] {
        let mut message = json!({"role":"assistant", "content":null, "tool_calls":[{"id":"call_test","type":"function","function":{"name":"probe","arguments":"{}"}}]});
        if provider == Provider::OpenRouter {
            message["reasoning_details"] = json!([{"type":"reasoning.encrypted","id":"rs_test","data":"encrypted-test","format":"anthropic-claude-v1","index":0}]);
        } else {
            message["reasoning_content"] = json!("test reasoning");
        }
        let (url, server) = mock_server(vec![
            chat_result(&message, "tool_calls"),
            chat_result(&json!({"role":"assistant","content":"done"}), "stop"),
        ])
        .await;
        let builder = agent_builder_at(
            provider,
            provider.default_model(),
            Some("test-key"),
            Some(&url),
        )
        .expect("builder");
        let agent = apply_completion_params::<_, std::collections::hash_map::RandomState>(
            builder,
            provider,
            provider.default_model(),
            4096,
            None,
            CompletionProfile::MainAgent,
        )
        .tool(Probe)
        .build();
        assert_eq!(
            agent.prompt("test").max_turns(3).await.expect("tool loop"),
            "done"
        );
        let requests = server.await.expect("mock task");
        assert!(requests[0].0.starts_with("POST /chat/completions "));
        assert_eq!(requests[0].1["model"], provider.default_model());
        assert_eq!(requests[0].1["max_tokens"], 4096);
        let messages = requests[1].1["messages"].as_array().expect("messages");
        assert!(
            messages
                .iter()
                .any(|item| item["role"] == "tool" && item["tool_call_id"] == "call_test")
        );
        let assistant = messages
            .iter()
            .find(|item| item["role"] == "assistant")
            .expect("assistant");
        if provider == Provider::OpenRouter {
            assert_eq!(assistant["reasoning_details"][0]["data"], "encrypted-test");
            assert_eq!(requests[0].1["reasoning"]["effort"], "high");
        } else {
            assert_eq!(assistant["reasoning_content"], "test reasoning");
        }
    }
}

#[tokio::test]
async fn anthropic_caching_and_effort_reach_the_messages_endpoint() {
    let response = json!({"id":"msg_test","type":"message","role":"assistant","model":"claude-opus-5","content":[{"type":"text","text":"done"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":1,"output_tokens":1}});
    let (url, server) = mock_server(vec![response]).await;
    let builder = agent_builder_at(
        Provider::Anthropic,
        "claude-opus-5",
        Some("test-key"),
        Some(&url),
    )
    .expect("builder");
    let agent = apply_completion_params::<_, std::collections::hash_map::RandomState>(
        builder,
        Provider::Anthropic,
        "claude-opus-5",
        4096,
        None,
        CompletionProfile::MainAgent,
    )
    .build();
    assert_eq!(agent.prompt("test").await.expect("prompt"), "done");
    let requests = server.await.expect("mock task");
    assert!(requests[0].0.starts_with("POST /v1/messages "));
    assert_eq!(requests[0].1["thinking"]["type"], "adaptive");
    assert_eq!(requests[0].1["output_config"]["effort"], "high");
    assert_eq!(requests[0].1["cache_control"]["type"], "ephemeral");
}

#[tokio::test]
async fn gemini_thinking_and_output_budget_reach_generate_content() {
    let response = json!({"candidates":[{"content":{"role":"model","parts":[{"text":"done"}]},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":1,"candidatesTokenCount":1,"totalTokenCount":2}});
    let (url, server) = mock_server(vec![response]).await;
    let builder = agent_builder_at(
        Provider::Google,
        "gemini-3.8-flash",
        Some("test-key"),
        Some(&url),
    )
    .expect("builder");
    let agent = apply_completion_params::<_, std::collections::hash_map::RandomState>(
        builder,
        Provider::Google,
        "gemini-3.8-flash",
        4096,
        None,
        CompletionProfile::Subagent,
    )
    .build();
    assert_eq!(agent.prompt("test").await.expect("prompt"), "done");
    let requests = server.await.expect("mock task");
    assert!(requests[0].0.contains("gemini-3.8-flash:generateContent"));
    assert_eq!(
        requests[0].1["generationConfig"]["thinkingConfig"]["thinkingLevel"],
        "low"
    );
    assert_eq!(requests[0].1["generationConfig"]["maxOutputTokens"], 4096);
}
