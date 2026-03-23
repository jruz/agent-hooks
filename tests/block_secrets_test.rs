mod helpers;

use std::collections::HashMap;

use helpers::{blocked_reason, payload, run_pre_tool_use};
use serde_json::Value;

fn file_payload(tool_name: &str, file_path: &str) -> helpers::PreToolUsePayload {
    let mut input = HashMap::new();
    input.insert(
        "file_path".to_string(),
        Value::String(file_path.to_string()),
    );
    payload(tool_name, input)
}

#[test]
fn blocks_read_env_file() {
    let output = run_pre_tool_use(&file_payload("Read", ".env"));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to sensitive file blocked"
    );
}

#[test]
fn blocks_read_ssh_key() {
    let output = run_pre_tool_use(&file_payload("Read", "/home/user/.ssh/id_rsa"));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to sensitive file blocked"
    );
}

#[test]
fn allows_read_envrc() {
    let output = run_pre_tool_use(&file_payload("Read", ".envrc"));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_read_envrc_in_path() {
    let output = run_pre_tool_use(&file_payload("Read", "/home/user/project/.envrc"));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_read_encrypted_env() {
    let output = run_pre_tool_use(&file_payload("Read", ".env.production.yaml"));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_read_ssh_config() {
    let output = run_pre_tool_use(&file_payload("Read", "/home/user/.ssh/config"));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_grep_normal_file() {
    let output = run_pre_tool_use(&file_payload("Grep", "src/main.rs"));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn blocks_read_env_in_path() {
    let output = run_pre_tool_use(&file_payload("Read", "/app/.env"));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to sensitive file blocked"
    );
}

#[test]
fn blocks_grep_ssh_key() {
    let output = run_pre_tool_use(&file_payload("Grep", "/home/user/.ssh/id_ed25519"));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to sensitive file blocked"
    );
}

#[test]
fn blocks_read_etc_shadow() {
    let output = run_pre_tool_use(&file_payload("Read", "/etc/shadow"));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to system secret file blocked"
    );
}

#[test]
fn blocks_read_etc_passwd() {
    let output = run_pre_tool_use(&file_payload("Read", "/etc/passwd"));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to system secret file blocked"
    );
}

#[test]
fn blocks_grep_etc_gshadow() {
    let output = run_pre_tool_use(&file_payload("Grep", "/etc/gshadow"));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to system secret file blocked"
    );
}

#[test]
fn blocks_read_etc_sudoers() {
    let output = run_pre_tool_use(&file_payload("Read", "/etc/sudoers"));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to system secret file blocked"
    );
}

#[test]
fn blocks_read_etc_master_passwd() {
    let output = run_pre_tool_use(&file_payload("Read", "/etc/master.passwd"));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to system secret file blocked"
    );
}
