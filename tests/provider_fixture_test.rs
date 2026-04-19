#[path = "support/provider_fixture.rs"]
mod helpers;

use std::fs;

use helpers::{
    blocked_reason, path_with_bin, run_fixture, run_hook_in_dir_no_agent, write_executable,
};
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn allows_claude_grep_fixture_with_path_field() {
    let output = run_fixture(
        "pre-tool-use",
        "claude",
        "claude/pre_tool_use/grep_path.json",
    );
    assert!(output.status.success());
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn blocks_claude_read_fixture_for_sensitive_file() {
    let output = run_fixture(
        "pre-tool-use",
        "claude",
        "claude/pre_tool_use/read_env.json",
    );
    let json = output_json(&output);

    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to sensitive file blocked"
    );
    assert_eq!(
        json,
        serde_json::json!({
            "decision": "block",
            "reason": "Access to sensitive file blocked",
        })
    );
}

#[test]
fn allows_stop_when_actions_pass() {
    let temp = setup_stop_project("#!/usr/bin/env bash\nexit 0\n", None, false);
    let bin_dir = temp.path().join("bin");
    let bin_path = bin_dir.as_path();
    let path = path_with_bin(bin_path);
    let temp_path = temp.path();
    let envs = [("PATH", path.as_str())];

    let output = run_hook_in_dir_no_agent("stop", "", temp_path, &envs);

    assert!(output.status.success());
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn blocks_stop_when_slop_detector_fails() {
    let temp = setup_stop_project(
        "#!/usr/bin/env bash\necho 'fixture slop failure' >&2\nexit 1\n",
        None,
        false,
    );
    let bin_dir = temp.path().join("bin");
    let bin_path = bin_dir.as_path();
    let path = path_with_bin(bin_path);
    let temp_path = temp.path();
    let envs = [("PATH", path.as_str())];

    let output = run_hook_in_dir_no_agent("stop", "{not-json", temp_path, &envs);

    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "slop-detector failed:\nfixture slop failure"
    );
}

#[test]
fn allows_stop_when_legacy_claude_trees_env_is_set() {
    let temp = setup_stop_project("#!/usr/bin/env bash\nexit 0\n", None, false);
    let bin_dir = temp.path().join("bin");
    let bin_path = bin_dir.as_path();
    let path = path_with_bin(bin_path);
    let temp_path = temp.path();
    let envs = [
        ("PATH", path.as_str()),
        ("CLAUDE_TREES_SIGNAL_DIR", "/definitely/missing"),
        ("CLAUDE_TREES_TAB_ID", "tab-123"),
    ];

    let output = run_hook_in_dir_no_agent("stop", "{not-json", temp_path, &envs);

    assert!(output.status.success());
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn blocks_stop_when_just_check_fails_without_parsing_stdin() {
    let just_script = "#!/usr/bin/env bash\nif [ \"$1\" = \"fix\" ]; then exit 0; fi\nif [ \"$1\" = \"check\" ]; then echo 'fixture just check failure'; exit 1; fi\nexit 0\n";
    let just_script = Some(just_script);
    let temp = setup_stop_project("#!/usr/bin/env bash\nexit 0\n", just_script, true);
    let bin_dir = temp.path().join("bin");
    let bin_path = bin_dir.as_path();
    let path = path_with_bin(bin_path);
    let temp_path = temp.path();
    let envs = [("PATH", path.as_str())];

    let output = run_hook_in_dir_no_agent("stop", "{not-json", temp_path, &envs);

    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "just check failed:\nfixture just check failure"
    );
}

#[test]
fn blocks_stop_when_just_fix_fails() {
    let just_script = "#!/usr/bin/env bash\nif [ \"$1\" = \"fix\" ]; then echo 'fixture just fix failure'; exit 1; fi\nif [ \"$1\" = \"check\" ]; then exit 0; fi\nexit 0\n";
    let just_script = Some(just_script);
    let temp = setup_stop_project("#!/usr/bin/env bash\nexit 0\n", just_script, true);
    let bin_dir = temp.path().join("bin");
    let bin_path = bin_dir.as_path();
    let path = path_with_bin(bin_path);
    let temp_path = temp.path();
    let envs = [("PATH", path.as_str())];

    let output = run_hook_in_dir_no_agent("stop", "{not-json", temp_path, &envs);

    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "just fix failed:\nfixture just fix failure"
    );
}

fn setup_stop_project(
    slop_script: &str,
    just_script: Option<&str>,
    with_justfile: bool,
) -> TempDir {
    let temp = tempfile::tempdir().expect("create temp dir");
    let bin_dir = temp.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("create bin dir");

    let slop_detector = bin_dir.join("slop-detector");
    write_executable(&slop_detector, slop_script);

    if let Some(script) = just_script {
        let just = bin_dir.join("just");
        write_executable(&just, script);
    }

    if with_justfile {
        let justfile = temp.path().join("justfile");
        fs::write(justfile, "").expect("write justfile");
    }

    temp
}

fn output_json(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("parse stdout as JSON")
}
