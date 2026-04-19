use serde::Serialize;

#[derive(Serialize)]
struct BlockResponse {
    decision: &'static str,
    reason: String,
}

#[derive(Serialize)]
struct PreToolUseDenyResponse {
    #[serde(rename = "hookSpecificOutput")]
    hook_specific_output: PreToolUseHookSpecificOutput,
}

#[derive(Serialize)]
struct PreToolUseHookSpecificOutput {
    #[serde(rename = "hookEventName")]
    hook_event_name: &'static str,
    #[serde(rename = "permissionDecision")]
    permission_decision: &'static str,
    #[serde(rename = "permissionDecisionReason")]
    permission_decision_reason: String,
}

pub fn stop_block(reason: String) -> Result<String, serde_json::Error> {
    let response = BlockResponse {
        decision: "block",
        reason,
    };
    serde_json::to_string(&response)
}

pub fn pre_tool_use_claude_block(reason: String) -> Result<String, serde_json::Error> {
    stop_block(reason)
}

pub fn pre_tool_use_codex_block(reason: String) -> Result<String, serde_json::Error> {
    let response = PreToolUseDenyResponse {
        hook_specific_output: PreToolUseHookSpecificOutput {
            hook_event_name: "PreToolUse",
            permission_decision: "deny",
            permission_decision_reason: reason,
        },
    };
    serde_json::to_string(&response)
}
