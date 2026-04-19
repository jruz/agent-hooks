#[path = "support/pre_tool_use.rs"]
mod helpers;

use std::collections::HashMap;
use std::process::Output;

use helpers::{blocked_reason, payload, run_pre_tool_use};
use serde_json::Value;

fn file_payload(tool_name: &str, file_path: &str) -> helpers::PreToolUsePayload {
    let mut input = HashMap::new();
    let key = "file_path".to_string();
    let file_path_value = file_path.to_string();
    let value = Value::String(file_path_value);
    input.insert(key, value);
    payload(tool_name, input)
}

fn run_file(tool_name: &str, file_path: &str) -> Output {
    let input = file_payload(tool_name, file_path);
    run_pre_tool_use(&input)
}

#[test]
fn blocks_read_env_file() {
    let output = run_file("Read", ".env");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to sensitive file blocked"
    );
}

#[test]
fn blocks_read_ssh_key() {
    let output = run_file("Read", "/home/user/.ssh/id_rsa");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to sensitive file blocked"
    );
}

#[test]
fn allows_read_envrc() {
    let output = run_file("Read", ".envrc");
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_read_envrc_in_path() {
    let output = run_file("Read", "/home/user/project/.envrc");
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_read_encrypted_env() {
    let output = run_file("Read", ".env.production.yaml");
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_read_ssh_config() {
    let output = run_file("Read", "/home/user/.ssh/config");
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_grep_normal_file() {
    let output = run_file("Grep", "src/main.rs");
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_grep_path_field() {
    let mut input = HashMap::new();
    let key = "path".to_string();
    let path = "src/main.rs".to_string();
    let value = Value::String(path);
    input.insert(key, value);
    let grep_payload = payload("Grep", input);
    let output = run_pre_tool_use(&grep_payload);
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn blocks_read_env_in_path() {
    let output = run_file("Read", "/app/.env");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to sensitive file blocked"
    );
}

#[test]
fn blocks_grep_ssh_key() {
    let output = run_file("Grep", "/home/user/.ssh/id_ed25519");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to sensitive file blocked"
    );
}

#[test]
fn blocks_read_etc_shadow() {
    let output = run_file("Read", "/etc/shadow");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to system secret file blocked"
    );
}

#[test]
fn blocks_read_etc_passwd() {
    let output = run_file("Read", "/etc/passwd");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to system secret file blocked"
    );
}

#[test]
fn blocks_grep_etc_gshadow() {
    let output = run_file("Grep", "/etc/gshadow");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to system secret file blocked"
    );
}

#[test]
fn blocks_read_etc_sudoers() {
    let output = run_file("Read", "/etc/sudoers");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to system secret file blocked"
    );
}

#[test]
fn blocks_read_etc_master_passwd() {
    let output = run_file("Read", "/etc/master.passwd");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to system secret file blocked"
    );
}
