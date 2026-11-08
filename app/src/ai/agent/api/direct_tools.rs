use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;
use warp_multi_agent_api::{self as api, message};

use super::RequestParams;
use crate::ai::agent::{AIAgentActionResultType, AIAgentInput};

#[cfg(test)]
#[path = "direct_tools_tests.rs"]
mod tests;
pub(super) const MAX_TOOL_ROUNDS: usize = 8;
const MAX_TOOL_OUTPUT: usize = 32 * 1024;

pub(super) fn may_request_command(params: &RequestParams, rounds: usize) -> bool {
    use crate::ai::agent::RequestCommandOutputResult;
    rounds < MAX_TOOL_ROUNDS && !params.input.iter().any(|input| matches!(input,
        AIAgentInput::ActionResult { result, .. }
        if matches!(result.result, AIAgentActionResultType::RequestCommandOutput(
            RequestCommandOutputResult::CancelledBeforeExecution
            | RequestCommandOutputResult::Denylisted { .. }
            | RequestCommandOutputResult::TerminalBusy { .. }
            | RequestCommandOutputResult::LongRunningCommandSnapshot { .. }
        ))
    ))
}

pub(super) fn definitions() -> Value {
    json!([{"type":"function","function":{
        "name":"run_shell_command",
        "description":"Request a command in the current terminal. The user must approve it before execution. Use the current shell and directory. Prefer short, finite commands and the smallest operation needed. Never start an interactive process or a background service.",
        "parameters":{"type":"object","properties":{"command":{"type":"string"}},"required":["command"],"additionalProperties":false}
    }}])
}

fn bounded(mut text: String) -> String {
    secret_redaction::redact_secrets(&mut text);
    if text.len() > MAX_TOOL_OUTPUT {
        let mut end = MAX_TOOL_OUTPUT;
        while !text.is_char_boundary(end) { end -= 1; }
        text.truncate(end);
        text.push_str("\n[Output truncated by WireTerm]");
    }
    text
}

pub(super) fn history(params: &RequestParams, messages: &mut Vec<Value>) -> anyhow::Result<usize> {
    let root = params.tasks.iter().find(|task| task.dependencies.as_ref()
        .is_none_or(|dependencies| dependencies.parent_task_id.is_empty()));
    let mut rounds = 0;
    let mut resolved = std::collections::HashSet::new();
    if let Some(root) = root {
        for entry in &root.messages {
            match &entry.message {
                Some(message::Message::UserQuery(query)) => {
                    rounds = 0;
                    messages.push(json!({"role":"user","content":query.query}));
                }
                Some(message::Message::AgentOutput(output)) => messages.push(json!({"role":"assistant","content":output.text})),
                Some(message::Message::ToolCall(call)) => {
                    let Some(message::tool_call::Tool::RunShellCommand(command)) = &call.tool else {
                        anyhow::bail!("This conversation contains unsupported tools. Start a new conversation.");
                    };
                    rounds += 1;
                    messages.push(json!({"role":"assistant","content":"","tool_calls":[{
                        "id":call.tool_call_id,"type":"function","function":{
                            "name":"run_shell_command","arguments":json!({"command":command.command}).to_string()
                        }
                    }]}));
                }
                Some(message::Message::ToolCallResult(result)) => {
                    resolved.insert(result.tool_call_id.clone());
                    messages.push(json!({"role":"tool","tool_call_id":result.tool_call_id,
                        "content":bounded(format!("{:?}", result.result))}));
                }
                _ => {}
            }
        }
    }
    for input in &params.input {
        match input {
            AIAgentInput::UserQuery { .. } => { rounds = 0; }
            AIAgentInput::ActionResult { result, .. } => {
                anyhow::ensure!(matches!(result.result, AIAgentActionResultType::RequestCommandOutput(_)),
                    "Direct Agent received an unsupported tool result.");
                let id = result.id.to_string();
                if resolved.insert(id.clone()) {
                    messages.push(json!({"role":"tool","tool_call_id":id,
                        "content":bounded(format!("{:?}",result.result))}));
                }
            }
            _ => {}
        }
    }
    Ok(rounds)
}

#[derive(Default)]
pub(super) struct PendingCommand {
    name: String,
    arguments: String,
    received: bool,
}

impl PendingCommand {
    pub(super) fn extend(&mut self, frame: &[u8]) -> anyhow::Result<()> {
        let frame = std::str::from_utf8(frame)?;
        let data = frame.lines().filter_map(|line| line.strip_prefix("data:").map(str::trim_start))
            .collect::<Vec<_>>().join("\n");
        if data.is_empty() || data.trim() == "[DONE]" { return Ok(()); }
        let chunk: Value = serde_json::from_str(&data)
            .map_err(|_| anyhow::anyhow!("Model returned invalid tool data."))?;
        let Some(calls) = chunk["choices"][0]["delta"]["tool_calls"].as_array() else { return Ok(()); };
        for call in calls {
            anyhow::ensure!(call["index"].as_u64() == Some(0), "Only one command may be requested per turn.");
            self.received = true;
            if let Some(name) = call["function"]["name"].as_str() { self.name.push_str(name); }
            if let Some(arguments) = call["function"]["arguments"].as_str() { self.arguments.push_str(arguments); }
            anyhow::ensure!(self.name.len() <= 64 && self.arguments.len() <= 16384, "Model tool request exceeded the size limit.");
        }
        Ok(())
    }

    pub(super) fn has_call(&self) -> bool { self.received }

    pub(super) fn message(self, task_id: &str, request_id: &str) -> anyhow::Result<api::Message> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Arguments { command: String }
        anyhow::ensure!(self.received && self.name == "run_shell_command", "Model requested an unsupported tool.");
        let args: Arguments = serde_json::from_str(&self.arguments)
            .map_err(|_| anyhow::anyhow!("Model returned invalid command arguments."))?;
        anyhow::ensure!(!args.command.trim().is_empty() && !args.command.contains('\0') && args.command.len() <= 8192,
            "Model returned an invalid command.");
        let id = Uuid::new_v4().to_string();
        #[allow(deprecated)]
        let command = message::tool_call::RunShellCommand {
            command: args.command, is_read_only: false, is_risky: true,
            ..Default::default()
        };
        Ok(api::Message { id: Uuid::new_v4().to_string(), task_id: task_id.into(), request_id: request_id.into(),
            message: Some(message::Message::ToolCall(message::ToolCall {
                tool_call_id: id, tool: Some(message::tool_call::Tool::RunShellCommand(command)),
            })), ..Default::default() })
    }
}
