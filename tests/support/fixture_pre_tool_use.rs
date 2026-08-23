use std::fs;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Output, Stdio};

pub fn run_fixture(agent: &str, relative_path: &str) -> Output {
    let path = fixture_path(relative_path);
    let stdin = fs::read_to_string(path).expect("read fixture");
    run_pre_tool_use_json(agent, &stdin)
}

pub fn blocked_reason(output: &Output) -> Option<String> {
    let parsed = stdout_json(output)?;

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

pub fn stdout_json(output: &Output) -> Option<serde_json::Value> {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return None;
    }

    let parsed = serde_json::from_str(&stdout).expect("parse stdout as JSON");
    Some(parsed)
}

fn fixture_path(relative_path: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(relative_path)
}

fn run_pre_tool_use_json(agent: &str, stdin_json: &str) -> Output {
    let mut tmp = tempfile::NamedTempFile::new().expect("create temp file");
    let stdin_bytes = stdin_json.as_bytes();
    tmp.write_all(stdin_bytes).expect("write temp file");
    let tmp_path = tmp.into_temp_path();

    let stdin_file = std::fs::File::open(tmp_path).expect("open temp file");
    let binary = env!("CARGO_BIN_EXE_agent-hooks");
    let stdin = Stdio::from(stdin_file);
    Command::new(binary)
        .arg("pre-tool-use")
        .arg("--agent")
        .arg(agent)
        .stdin(stdin)
        .output()
        .expect("run binary")
}
