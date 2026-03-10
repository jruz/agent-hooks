mod helpers;

use std::collections::HashMap;

use helpers::{blocked_reason, payload, run_pre_tool_use};
use serde_json::Value;

fn bash(command: &str) -> helpers::PreToolUsePayload {
    let mut input = HashMap::new();
    input.insert("command".to_string(), Value::String(command.to_string()));
    payload("Bash", input)
}

#[test]
fn allows_non_commit_command() {
    let output = run_pre_tool_use(&bash("git push origin main"));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_no_edit_commit() {
    let output = run_pre_tool_use(&bash("git commit --no-edit"));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_fixup_commit() {
    let output = run_pre_tool_use(&bash("git commit --fixup abc123"));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_valid_feat_commit() {
    let output = run_pre_tool_use(&bash(r#"git commit -m "[feat] Auth: add login""#));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_valid_bugs_commit() {
    let output = run_pre_tool_use(&bash(r#"git commit -m "[bugs] Fix: resolve crash""#));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_valid_refa_commit() {
    let output = run_pre_tool_use(&bash(r#"git commit -m "[refa] Config: update settings""#));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_valid_deps_commit() {
    let output = run_pre_tool_use(&bash(r#"git commit -m "[deps] Cargo: bump serde""#));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_valid_tool_commit() {
    let output = run_pre_tool_use(&bash(r#"git commit -m "[tool] CI: add check""#));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn blocks_unknown_tag() {
    let output = run_pre_tool_use(&bash(r#"git commit -m "[chore] Area: description""#));
    assert!(blocked_reason(&output).is_some());
}

#[test]
fn blocks_missing_brackets() {
    let output = run_pre_tool_use(&bash(r#"git commit -m "feat Area: description""#));
    assert!(blocked_reason(&output).is_some());
}

#[test]
fn blocks_missing_colon() {
    let output = run_pre_tool_use(&bash(r#"git commit -m "[feat] Area description""#));
    assert!(blocked_reason(&output).is_some());
}

#[test]
fn blocks_missing_area() {
    let output = run_pre_tool_use(&bash(r#"git commit -m "[feat] : description""#));
    assert!(blocked_reason(&output).is_some());
}

#[test]
fn blocks_missing_description_after_colon() {
    let output = run_pre_tool_use(&bash(r#"git commit -m "[feat] Area:""#));
    assert!(blocked_reason(&output).is_some());
}

#[test]
fn allows_valid_heredoc_commit() {
    let output = run_pre_tool_use(&bash(
        "git commit -m \"$(cat <<'EOF'\n[feat] Auth: add login\n\nCo-Authored-By: Claude <noreply@anthropic.com>\nEOF\n)\"",
    ));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_cd_prefix_commit() {
    let output = run_pre_tool_use(&bash(
        r#"cd /Users/jruz/dev/dotfiles && git commit -m "[bugs] Deff: Update source hash""#,
    ));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_multi_word_area() {
    let output = run_pre_tool_use(&bash(r#"git commit -m "[feat] Auth module: add login""#));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn blocks_bare_commit_no_message() {
    let output = run_pre_tool_use(&bash("git commit"));
    assert!(blocked_reason(&output).is_some());
}
