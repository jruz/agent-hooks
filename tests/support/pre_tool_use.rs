use std::collections::HashMap;
use std::io::Write as _;
use std::process::{Command, Output};

use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub struct PreToolUsePayload {
    pub cwd: String,
    pub hook_event_name: String,
    pub session_id: String,
    pub tool_input: HashMap<String, Value>,
    pub tool_name: String,
    pub tool_use_id: String,
    pub transcript_path: String,
}

pub fn payload(tool_name: &str, tool_input: HashMap<String, Value>) -> PreToolUsePayload {
    PreToolUsePayload {
        cwd: "/tmp".to_string(),
        hook_event_name: "PreToolUse".to_string(),
        session_id: "test".to_string(),
        tool_input,
        tool_name: tool_name.to_string(),
        tool_use_id: "test-id".to_string(),
        transcript_path: "/tmp/test".to_string(),
    }
}

pub fn run_pre_tool_use(payload: &PreToolUsePayload) -> Output {
    let json = serde_json::to_string(payload).expect("serialize payload");
    run_pre_tool_use_json(&json)
}

pub fn blocked_reason(output: &Output) -> Option<String> {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return None;
    }
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("parse stdout as JSON");
    parsed
        .get("reason")
        .and_then(serde_json::Value::as_str)
        .map(String::from)
        .or_else(|| {
            parsed
                .get("hookSpecificOutput")
                .and_then(|value| value.get("permissionDecisionReason"))
                .and_then(serde_json::Value::as_str)
                .map(String::from)
        })
}

fn run_pre_tool_use_json(stdin_json: &str) -> Output {
    let mut tmp = tempfile::NamedTempFile::new().expect("create temp file");
    let stdin_bytes = stdin_json.as_bytes();
    tmp.write_all(stdin_bytes).expect("write temp file");
    let tmp_path = tmp.into_temp_path();

    let stdin_file = std::fs::File::open(tmp_path).expect("open temp file");
    let binary = env!("CARGO_BIN_EXE_agent-hooks");
    Command::new(binary)
        .arg("pre-tool-use")
        .arg("--agent")
        .arg("claude")
        .stdin(stdin_file)
        .output()
        .expect("run binary")
}
