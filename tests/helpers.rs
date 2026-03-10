use std::collections::HashMap;
use std::io::Write as _;
use std::process::{Command, Output, Stdio};

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

pub fn run_pre_tool_use(p: &PreToolUsePayload) -> Output {
    let json = serde_json::to_string(p).expect("serialize payload");

    let mut tmp = tempfile::NamedTempFile::new().expect("create temp file");
    tmp.write_all(json.as_bytes()).expect("write temp file");
    let tmp_path = tmp.into_temp_path();

    let stdin_file = std::fs::File::open(&tmp_path).expect("open temp file");
    Command::new(env!("CARGO_BIN_EXE_claude-hooks"))
        .arg("pre-tool-use")
        .stdin(Stdio::from(stdin_file))
        .output()
        .expect("run binary")
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
}
