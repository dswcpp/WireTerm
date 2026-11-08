use super::*;

fn params(base_url: &str) -> RequestParams {
    use api::request::settings::CustomModelProviders;
    use api::request::settings::custom_model_providers::{CustomModel, CustomModelProvider};
    let mut params = RequestParams::new_for_test();
    params.custom_model_providers = Some(CustomModelProviders {
        providers: vec![CustomModelProvider {
            base_url: base_url.into(),
            api_key: String::new(),
            schema: 0,
            models: vec![CustomModel {
                slug: "my-model".into(),
                config_key: "test-model".into(),
                ..Default::default()
            }],
        }],
    });
    params.input.push(AIAgentInput::UserQuery {
        query: "Explain this log".into(),
        context: Arc::from([AIAgentContext::SelectedText("device ready".into())]),
        static_query_type: None,
        referenced_attachments: Default::default(),
        user_query_mode: Default::default(),
        running_command: None,
        intended_agent: None,
        base: None,
    });
    params
}

#[test]
fn command_suggestions_reject_prose_empty_and_invalid_commands() {
    assert!(parse_commands("Run ls").is_err());
    assert!(parse_commands("[]").is_err());
    assert!(parse_commands(r#"[{"command":"","description":"empty"}]"#).is_err());
    assert!(parse_commands(r#"[{"command":"echo \u0000","description":"invalid"}]"#).is_err());
    assert!(parse_commands(r#"[{"command":"Get-ChildItem","description":"List files"}]"#).is_ok());
}

#[test]
fn endpoint_url_preserves_prefix_and_rejects_embedded_secrets() {
    assert_eq!(
        completion_url("https://provider.example/api/v1/").unwrap(),
        "https://provider.example/api/v1/chat/completions"
    );
    assert_eq!(
        completion_url("http://127.0.0.1:11434/v1/chat/completions").unwrap(),
        "http://127.0.0.1:11434/v1/chat/completions"
    );
    assert!(completion_url("http://[::1]:11434/v1").is_ok());
    for url in [
        "http://provider.example/v1",
        "https://user:secret@provider.example/v1",
        "https://provider.example/v1?key=secret",
        "https://provider.example/v1#secret",
        "file:///tmp/model",
    ] {
        assert!(completion_url(url).is_err());
    }
}

#[test]
fn request_uses_selected_model_history_and_explicit_text_only() {
    let mut params = params("http://localhost:11434/v1");
    params.tasks.push(api::Task {
        id: "existing-task".into(),
        messages: vec![api::Message {
            message: Some(message::Message::AgentOutput(message::AgentOutput {
                text: "Earlier answer".into(),
            })),
            ..Default::default()
        }],
        ..Default::default()
    });
    let prepared = prepare(&params, "request").unwrap();
    let body: serde_json::Value = serde_json::from_slice(&prepared.body).unwrap();
    assert_eq!(body["model"], "my-model");
    assert_eq!(body["messages"][1]["content"], "Earlier answer");
    assert!(
        body["messages"][2]["content"]
            .as_str()
            .unwrap()
            .contains("device ready")
    );
    assert!(body.get("tools").is_none());
    assert!(body.get("api_keys").is_none());
    assert!(!prepared.new_task);
    assert_eq!(prepared.task_id, "existing-task");
}

#[test]
fn missing_model_and_unsupported_requests_fail_without_fallback() {
    let mut params = params("http://localhost:11434/v1");
    params.model = "missing".into();
    assert!(prepare(&params, "request").is_err());
    params.model = "test-model".into();
    params.input = vec![AIAgentInput::ResumeConversation {
        context: Arc::from([]),
    }];
    assert!(prepare(&params, "request").is_err());
}

#[test]
fn remote_endpoint_requires_key_and_request_size_is_bounded() {
    assert!(prepare(&params("https://provider.example/v1"), "request").is_err());
    let mut params = params("http://localhost:11434/v1");
    let AIAgentInput::UserQuery { query, .. } = &mut params.input[0] else {
        panic!("expected query")
    };
    *query = "x".repeat(MAX_REQUEST_BYTES);
    assert!(prepare(&params, "request").is_err());
}

#[test]
fn parses_sse_text_and_rejects_tools_and_malformed_events() {
    assert_eq!(parse_event(b": keepalive").unwrap(), None);
    assert_eq!(
        parse_event(b"data: [DONE]").unwrap(),
        Some((String::new(), Some("done".into())))
    );
    let event =
        br#"data: {"choices":[{"index":0,"delta":{"content":"hello"},"finish_reason":null}]}"#;
    assert_eq!(parse_event(event).unwrap(), Some(("hello".into(), None)));
    assert!(parse_event(br#"data: {"choices":[{"index":0,"delta":{"tool_calls":[{}]},"finish_reason":"tool_calls"}]}"#).is_err());
    assert!(parse_event(b"data: not json").is_err());
}

#[cfg(not(target_family = "wasm"))]
fn serve(response: String) -> (String, std::thread::JoinHandle<String>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        let (mut connection, _) = listener.accept().unwrap();
        connection
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let mut request = Vec::new();
        let mut chunk = [0; 4096];
        loop {
            let count = connection.read(&mut chunk).unwrap();
            if count == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..count]);
            if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
                let length: usize = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length: "))
                    .unwrap()
                    .parse()
                    .unwrap();
                if request.len() >= end + 4 + length {
                    break;
                }
            }
        }
        for part in response.as_bytes().chunks(7) {
            connection.write_all(part).unwrap();
        }
        String::from_utf8(request).unwrap()
    });
    (url, handle)
}

#[test]
#[cfg(not(target_family = "wasm"))]
fn streams_from_loopback_without_warp_headers_or_credentials() {
    let body = "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"你好\"},\"finish_reason\":null}]}\r\n\r\ndata: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\r\n\r\n";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let (url, server) = serve(response);
    let (_cancel, receiver) = oneshot::channel();
    let events =
        futures_lite::future::block_on(generate(params(&url), receiver).collect::<Vec<_>>());
    assert!(events.iter().all(Result::is_ok), "{events:?}");
    assert!(matches!(
        events.last().unwrap().as_ref().unwrap().r#type,
        Some(response_event::Type::Finished(_))
    ));
    assert!(
        events
            .iter()
            .any(|event| format!("{event:?}").contains("你好"))
    );
    let request = server.join().unwrap().to_ascii_lowercase();
    assert!(request.starts_with("post /v1/chat/completions "));
    assert!(!request.contains("x-warp"));
    assert!(!request.contains("authorization:"));
}

#[test]
#[cfg(not(target_family = "wasm"))]
fn rejects_redirect_without_disclosing_response_body() {
    let (url, server) = serve("HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/secret\r\nContent-Length: 6\r\nConnection: close\r\n\r\nsecret".into());
    let (_cancel, receiver) = oneshot::channel();
    let events =
        futures_lite::future::block_on(generate(params(&url), receiver).collect::<Vec<_>>());
    let error = events[0].as_ref().unwrap_err().to_string();
    assert!(error.contains("302"));
    assert!(!error.contains("secret"));
    server.join().unwrap();
}

#[test]
#[cfg(not(target_family = "wasm"))]
fn command_suggestions_connect_without_an_existing_tokio_runtime() {
    let body = serde_json::json!({
        "choices": [{
            "finish_reason": "stop",
            "message": { "content": r#"[{"command":"Get-ChildItem","description":"List files"}]"# }
        }]
    }).to_string();
    let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
    let (url, server) = serve(response);
    let endpoint = ai::api_keys::CustomEndpoint {
        name: "test".into(), url, api_key: String::new(),
        schema: ai::api_keys::CustomEndpointSchema::OpenaiChatCompletions,
        models: Vec::new(),
    };
    let suggestions = futures_lite::future::block_on(generate_commands(
        endpoint, "model".into(), "list files".into(), None,
    )).unwrap();
    assert_eq!(suggestions.len(), 1);
    let request = server.join().unwrap();
    assert!(request.starts_with("POST /v1/chat/completions "));
    assert!(request.contains("\"stream\":false"));
}

#[test]
fn cancellation_before_polling_makes_no_request() {
    let (cancel, receiver) = oneshot::channel();
    cancel.send(()).unwrap();
    let events = futures_lite::future::block_on(
        generate(params("http://127.0.0.1:1/v1"), receiver).collect::<Vec<_>>(),
    );
    assert!(events.is_empty());
}
