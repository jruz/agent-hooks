#[path = "support/fixture_pre_tool_use.rs"]
mod helpers;

use helpers::{blocked_reason, run_fixture, stdout_json};
use serde_json::Value;

#[test]
fn blocks_codex_bash_env_file() {
    let output = run_fixture("codex", "codex/pre_tool_use/bash_env.json");
    let json = stdout_json(&output).expect("should emit json");
    let hook_specific = json
        .get("hookSpecificOutput")
        .and_then(Value::as_object)
        .expect("should emit hookSpecificOutput");

    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Never access .env files directly"
    );
    assert_eq!(
        hook_specific.get("hookEventName").and_then(Value::as_str),
        Some("PreToolUse")
    );
    assert_eq!(
        hook_specific
            .get("permissionDecision")
            .and_then(Value::as_str),
        Some("deny")
    );
    assert_eq!(
        hook_specific
            .get("permissionDecisionReason")
            .and_then(Value::as_str),
        Some("Never access .env files directly")
    );
    assert!(json.get("decision").is_none());
}

#[test]
fn allows_safe_codex_bash() {
    let output = run_fixture("codex", "codex/pre_tool_use/git_status.json");
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn ignores_non_bash_codex_tool() {
    let output = run_fixture("codex", "codex/pre_tool_use/read_env.json");
    assert!(blocked_reason(&output).is_none());
    assert!(stdout_json(&output).is_none());
}
