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
    assert_eq!(body["tools"][0]["function"]["name"], "run_shell_command");
    assert_eq!(body["parallel_tool_calls"], false);
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
fn parses_sse_text_and_tool_completion_and_rejects_malformed_events() {
    assert_eq!(parse_event(b": keepalive").unwrap(), None);
    assert_eq!(
        parse_event(b"data: [DONE]").unwrap(),
        Some((String::new(), Some("done".into())))
    );
    let event =
        br#"data: {"choices":[{"index":0,"delta":{"content":"hello"},"finish_reason":null}]}"#;
    assert_eq!(parse_event(event).unwrap(), Some(("hello".into(), None)));
    assert!(parse_event(br#"data: {"choices":[{"index":0,"delta":{"tool_calls":[]},"finish_reason":"tool_calls"}]}"#).is_ok());
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
    })
    .to_string();
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let (url, server) = serve(response);
    let endpoint = ai::api_keys::CustomEndpoint {
        name: "test".into(),
        url,
        api_key: String::new(),
        schema: ai::api_keys::CustomEndpointSchema::OpenaiChatCompletions,
        models: Vec::new(),
    };
    let suggestions = futures_lite::future::block_on(generate_commands(
        endpoint,
        "model".into(),
        "list files".into(),
        None,
    ))
    .unwrap();
    assert_eq!(suggestions.len(), 1);
    let request = server.join().unwrap();
    assert!(request.starts_with("POST /v1/chat/completions "));
    assert!(request.contains("\"stream\":false"));
}

#[test]
#[cfg(not(target_family = "wasm"))]
fn streamed_command_is_proposed_as_a_native_action_without_execution() {
    let body = concat!(
        "data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"name\":\"run_shell_command\",\"arguments\":\"{\\\"command\\\":\\\"Get-ChildItem\\\"}\"}}]},\"finish_reason\":null}]}\n\n",
        "data: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"tool_calls\"}]}\n\n"
    );
    let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
    let (url, server) = serve(response);
    let (_cancel, receiver) = oneshot::channel();
    let events = futures_lite::future::block_on(generate(params(&url), receiver).collect::<Vec<_>>());
    assert!(events.iter().all(Result::is_ok), "{events:?}");
    let proposed = events.into_iter().filter_map(|event| event.ok())
        .filter_map(|event| match event.r#type {
            Some(response_event::Type::ClientActions(actions)) => Some(actions.actions),
            _ => None,
        }).flatten().filter_map(|action| match action.action {
            Some(client_action::Action::AddMessagesToTask(add)) => Some(add.messages),
            _ => None,
        }).flatten().find_map(|message| match message.message {
            Some(message::Message::ToolCall(call)) => Some(call),
            _ => None,
        }).expect("The model command must be exposed as an approval-controlled native tool call");
    assert!(matches!(proposed.tool, Some(message::tool_call::Tool::RunShellCommand(command)) if command.command == "Get-ChildItem"));
    let request = server.join().unwrap();
    assert!(request.contains("run_shell_command"));
}

#[test]
#[cfg(not(target_family = "wasm"))]
fn rejected_command_returns_to_model_without_more_tool_authority() {
    use crate::ai::agent::{AIAgentActionResult, AIAgentActionResultType, RequestCommandOutputResult};
    let body = "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Cancelled\"},\"finish_reason\":\"stop\"}]}\n\n";
    let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
    let (url, server) = serve(response);
    let mut params = params(&url);
    let call_id = "approved-target-id";
    params.tasks = vec![api::Task {
        id: "task".into(),
        messages: vec![api::Message {
            message: Some(message::Message::ToolCall(message::ToolCall {
                tool_call_id: call_id.into(),
                tool: Some(message::tool_call::Tool::RunShellCommand(message::tool_call::RunShellCommand {
                    command: "Get-ChildItem".into(), ..Default::default()
                })),
            })), ..Default::default()
        }], ..Default::default()
    }];
    params.input = vec![AIAgentInput::ActionResult {
        result: AIAgentActionResult {
            id: call_id.to_string().into(),
            task_id: crate::ai::agent::task::TaskId::new("task".into()),
            result: AIAgentActionResultType::RequestCommandOutput(RequestCommandOutputResult::CancelledBeforeExecution),
        }, context: Arc::from([]),
    }];
    let (_cancel, receiver) = oneshot::channel();
    let events = futures_lite::future::block_on(generate(params, receiver).collect::<Vec<_>>());
    assert!(events.iter().all(Result::is_ok), "{events:?}");
    let request = server.join().unwrap();
    let body: serde_json::Value = serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(body["tool_choice"], "none");
    assert_eq!(body["messages"][2]["role"], "tool");
    assert_eq!(body["messages"][2]["tool_call_id"], "approved-target-id");
    assert!(body["messages"][2]["content"].as_str().unwrap().contains("CancelledBeforeExecution"));
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
