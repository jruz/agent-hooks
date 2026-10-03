#[path = "support/fixture_pre_tool_use.rs"]
mod helpers;

use helpers::{blocked_reason, run_fixture, stdout_json};
use serde_json::Value;

#[test]
fn blocks_pi_bash_with_native_response() {
    let output = run_fixture("pi", "pi/pre_tool_use/bash_env.json");
    let json = stdout_json(&output).expect("should emit json");

    assert_eq!(json.get("block"), Some(&Value::Bool(true)));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Never access .env files directly"
    );
    assert!(json.get("hookSpecificOutput").is_none());
    assert!(json.get("permissionDecision").is_none());
    assert!(json.get("decision").is_none());
}

#[test]
fn blocks_pi_read_env_file() {
    let output = run_fixture("pi", "pi/pre_tool_use/read_env.json");

    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to sensitive file blocked"
    );
}

#[test]
fn blocks_pi_grep_env_path() {
    let output = run_fixture("pi", "pi/pre_tool_use/grep_env.json");

    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to sensitive file blocked"
    );
}

#[test]
fn blocks_pi_read_docker_credentials() {
    let output = run_fixture("pi", "pi/pre_tool_use/read_docker_config.json");

    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to sensitive file blocked"
    );
}

#[test]
fn blocks_pi_grep_docker_directory() {
    let output = run_fixture("pi", "pi/pre_tool_use/grep_docker_directory.json");

    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to sensitive file blocked"
    );
}

#[test]
fn allows_safe_pi_bash() {
    let output = run_fixture("pi", "pi/pre_tool_use/git_status.json");

    assert!(blocked_reason(&output).is_none());
    assert!(stdout_json(&output).is_none());
}

#[test]
fn passes_unknown_pi_tools() {
    let output = run_fixture("pi", "pi/pre_tool_use/unknown.json");

    assert!(output.status.success());
    assert!(stdout_json(&output).is_none());
}

#[test]
fn reports_malformed_pi_json() {
    let output = run_fixture("pi", "pi/pre_tool_use/malformed.json");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success());
    assert!(stderr.contains("failed to parse Pi PreToolUse JSON"));
}
