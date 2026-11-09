use super::*;

#[test]
fn completed_context_does_not_block_local_shell_but_running_connections_do() {
    let command = PendingCommand {
        name: "run_shell_command".into(),
        arguments: r#"{"command":"dir"}"#.into(),
        received: true,
    };
    assert!(
        command
            .validate_target(&["completed-context".into()], false)
            .is_ok()
    );
    assert!(
        command
            .validate_target(&["running-connection".into()], true)
            .is_err()
    );
}

#[test]
fn terminal_writes_bind_exact_bytes_and_reject_connection_shortcuts() {
    let command = PendingCommand {
        name: "write_to_long_running_shell_command".into(),
        arguments: r#"{"command_id":"serial-block","input":"status\r"}"#.into(),
        received: true,
    };
    assert!(
        command
            .validate_target(&["other-block".into()], true)
            .is_err()
    );
    command
        .validate_target(&["serial-block".into()], true)
        .unwrap();
    let native = command.message("task", "request").unwrap();
    let Some(message::Message::ToolCall(call)) = native.message else {
        panic!("Expected a tool");
    };
    let Some(message::tool_call::Tool::WriteToLongRunningShellCommand(write)) = call.tool else {
        panic!("Expected terminal input");
    };
    assert_eq!(write.command_id, "serial-block");
    assert_eq!(write.input, b"status\r");
    assert!(write.mode.is_none());
    for input in ["\u{1d}break\r", "\u{11}", "\u{16}", "\0", ""] {
        let command = PendingCommand {
            name: "write_to_long_running_shell_command".into(),
            arguments: json!({"command_id":"serial-block","input":input}).to_string(),
            received: true,
        };
        assert!(command.message("task", "request").is_err());
    }
}

#[test]
fn terminal_reads_are_immediate_and_cannot_request_unbounded_waits() {
    let command = PendingCommand {
        name: "read_shell_command_output".into(),
        arguments: r#"{"command_id":"ssh-block"}"#.into(),
        received: true,
    };
    let native = command.message("task", "request").unwrap();
    let Some(message::Message::ToolCall(call)) = native.message else {
        panic!("Expected a tool");
    };
    let Some(message::tool_call::Tool::ReadShellCommandOutput(read)) = call.tool else {
        panic!("Expected a read");
    };
    assert_eq!(read.command_id, "ssh-block");
    assert!(matches!(
        read.delay,
        Some(
            message::tool_call::read_shell_command_output::Delay::Duration(prost_types::Duration {
                seconds: 0,
                nanos: 0
            })
        )
    ));
    let unbounded = PendingCommand {
        name: "read_shell_command_output".into(),
        arguments: r#"{"command_id":"ssh-block","delay":999999}"#.into(),
        received: true,
    };
    assert!(unbounded.message("task", "request").is_err());
}

#[test]
fn cancelled_terminal_input_stops_the_tool_loop() {
    use crate::ai::agent::{AIAgentActionResult, WriteToLongRunningShellCommandResult};
    let mut params = RequestParams::new_for_test();
    params.input.push(AIAgentInput::ActionResult {
        result: AIAgentActionResult {
            id: "write-call".to_owned().into(),
            task_id: crate::ai::agent::task::TaskId::new("task".into()),
            result: AIAgentActionResultType::WriteToLongRunningShellCommand(
                WriteToLongRunningShellCommandResult::Cancelled,
            ),
        },
        context: std::sync::Arc::from([]),
    });
    assert!(!may_request_command(&params, 1));
    let mut messages = Vec::new();
    history(&params, &mut messages).unwrap();
    assert_eq!(messages[0]["tool_call_id"], "write-call");
}

#[test]
fn an_exited_connection_cannot_fall_back_to_a_local_command() {
    use crate::ai::agent::{AIAgentActionResult, WriteToLongRunningShellCommandResult};
    let mut params = RequestParams::new_for_test();
    params.input.push(AIAgentInput::ActionResult {
        result: AIAgentActionResult {
            id: "closed-connection".to_owned().into(),
            task_id: crate::ai::agent::task::TaskId::new("task".into()),
            result: AIAgentActionResultType::WriteToLongRunningShellCommand(
                WriteToLongRunningShellCommandResult::CommandFinished {
                    block_id: "serial-block".to_owned().into(),
                    output: "Serial port released".into(),
                    exit_code: Default::default(),
                    start_ts: None,
                    completed_ts: None,
                },
            ),
        },
        context: std::sync::Arc::from([]),
    });
    assert!(!may_request_command(&params, 1));
}

#[test]
fn command_fragments_become_a_native_tool_action() {
    let mut command = PendingCommand::default();
    command.extend(br#"data: {"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"name":"run_shell_command","arguments":"{\"command\":\"Get-"}}]}}]}"#).unwrap();
    command.extend(br#"data: {"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"ChildItem\"}"}}]}}]}"#).unwrap();
    let native = command.message("task", "request").unwrap();
    let Some(message::Message::ToolCall(call)) = native.message else {
        panic!("Expected a tool call");
    };
    let Some(message::tool_call::Tool::RunShellCommand(command)) = call.tool else {
        panic!("Expected a shell command");
    };
    assert_eq!(command.command, "Get-ChildItem");
    assert!(!command.is_read_only);
    assert!(command.is_risky);
    assert_eq!(native.task_id, "task");
}

#[test]
fn tool_round_limit_requires_a_new_user_request() {
    let params = RequestParams::new_for_test();
    assert!(may_request_command(&params, 7));
    assert!(!may_request_command(&params, 8));
}

#[test]
fn parallel_and_unknown_tools_are_rejected() {
    let mut command = PendingCommand::default();
    assert!(
        command
            .extend(br#"data: {"choices":[{"index":0,"delta":{"tool_calls":[{"index":1}]}}]}"#)
            .is_err()
    );
    let unknown = PendingCommand {
        name: "delete_files".into(),
        arguments: "{}".into(),
        received: true,
    };
    assert!(unknown.message("task", "request").is_err());
}

#[test]
fn extra_arguments_cannot_change_the_execution_target() {
    let command = PendingCommand {
        name: "run_shell_command".into(),
        arguments: r#"{"command":"dir","cwd":"C:/Windows"}"#.into(),
        received: true,
    };
    assert!(command.message("task", "request").is_err());
}

#[test]
fn oversized_output_is_explicitly_truncated_at_a_character_boundary() {
    let output = bounded("设备".repeat(MAX_TOOL_OUTPUT));
    assert!(output.ends_with("[Output truncated by WireTerm]"));
    assert!(output.len() < MAX_TOOL_OUTPUT + 100);
}

#[test]
fn tool_results_are_returned_as_untrusted_tool_messages() {
    use crate::ai::agent::{AIAgentActionResult, RequestCommandOutputResult};
    let mut params = RequestParams::new_for_test();
    params.input.push(AIAgentInput::ActionResult {
        result: AIAgentActionResult {
            id: "call-id".to_string().into(),
            task_id: crate::ai::agent::task::TaskId::new("task".into()),
            result: AIAgentActionResultType::RequestCommandOutput(
                RequestCommandOutputResult::CancelledBeforeExecution,
            ),
        },
        context: std::sync::Arc::from([]),
    });
    let mut messages = Vec::new();
    history(&params, &mut messages).unwrap();
    assert_eq!(messages[0]["role"], "tool");
    assert_eq!(messages[0]["tool_call_id"], "call-id");
    assert!(
        messages[0]["content"]
            .as_str()
            .unwrap()
            .contains("CancelledBeforeExecution")
    );
    assert!(!may_request_command(&params, 1));
}

#[test]
fn legacy_function_call_becomes_an_approval_controlled_command() {
    let mut command = PendingCommand::default();
    command.extend(br#"data: {"choices":[{"index":0,"delta":{"function_call":{"name":"run_shell_command","arguments":"{\"command\":\"cmd.exe /c dir\"}"}}}]}"#).unwrap();
    let native = command.message("task", "request").unwrap();
    let Some(message::Message::ToolCall(call)) = native.message else {
        panic!("Expected a tool call");
    };
    let Some(message::tool_call::Tool::RunShellCommand(command)) = call.tool else {
        panic!("Expected a command");
    };
    assert_eq!(command.command, "cmd.exe /c dir");
    assert!(command.is_risky);
}

#[test]
fn other_choices_cannot_supply_a_command() {
    let mut command = PendingCommand::default();
    command.extend(br#"data: {"choices":[{"index":1,"delta":{"tool_calls":[{"index":0,"function":{"name":"run_shell_command","arguments":"{}"}}]}}]}"#).unwrap();
    assert!(!command.has_call());
}
