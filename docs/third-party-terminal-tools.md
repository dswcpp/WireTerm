# Third-party model terminal commands

Direct Agent exposes `run_shell_command` to models configured through an OpenAI-compatible
Chat Completions endpoint. The selected model and endpoint must support streaming function calls.
Text-only models can answer questions but cannot execute a command by printing it in a code block.

Enter Agent mode with `#`, then request a finite operation such as listing the current directory.
The model proposes a command for the active terminal's shell and working directory. Approve the
proposal to execute it. On Windows, PowerShell commands such as `Get-ChildItem` and explicit CMD
commands such as `cmd.exe /c dir` use the same terminal execution path.

The command output and exit status return to the model under the matching tool call ID. A completed
command can be followed by another proposed command, up to eight tool rounds per user request.
Cancellation, a denied command, a busy terminal, or a long-running command snapshot prevents further
command requests in that response. A new user request starts a new round budget.

The response adapter accepts streamed `tool_calls` and legacy `function_call` deltas. A complete
command may finish with `tool_calls`, `function_call`, `stop`, or the stream's completion marker.
Incomplete arguments, unknown tools, parallel commands, and token-limit termination are rejected
before a command is proposed for approval.

If listing the directory fails, distinguish these cases:

- No command proposal: check that the chosen model and endpoint support function calls.
- A proposal appears but does not execute: check the approval state and whether the terminal is busy.
- A command executes and reports an error: inspect its output and the active shell and directory.
- The adapter reports invalid tool arguments or a truncated response: check the provider's streaming
  tool-call response and output token limit.
