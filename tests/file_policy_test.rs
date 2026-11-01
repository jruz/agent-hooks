#[path = "support/hook_json.rs"]
mod helpers;

fn assert_file_policy(paths: &[&str], expected: Option<&str>) {
    for agent in ["claude", "pi"] {
        for tool in ["Read", "Grep"] {
            for path in paths {
                assert_file_call(agent, tool, path, expected);
            }
        }
    }
}

fn file_input(agent: &str, tool: &str, path: &str) -> serde_json::Value {
    match (agent, tool) {
        ("pi", "Read") => serde_json::json!({"toolName": "read", "input": {"path": path}}),
        ("pi", _) => {
            serde_json::json!({"toolName": "grep", "input": {"path": path, "pattern": "auth"}})
        }
        (_, "Read") => serde_json::json!({"tool_name": tool, "tool_input": {"file_path": path}}),
        (_, _) => {
            serde_json::json!({"tool_name": tool, "tool_input": {"path": path, "pattern": "auth"}})
        }
    }
}

fn assert_file_call(agent: &str, tool: &str, path: &str, expected: Option<&str>) {
    let input = file_input(agent, tool, path);
    let json = input.to_string();
    let output = helpers::run_pre_tool_use_json(agent, &json);
    let label = format!("{agent}: {tool}: {path}");
    assert!(output.status.success(), "{label}");
    assert!(output.stderr.is_empty(), "{label}");
    let reason = helpers::blocked_reason(&output);
    assert_eq!(reason.as_deref(), expected, "{label}");
    let response = helpers::stdout_json(&output);
    if expected.is_some() {
        let response = response.expect("blocked response");
        assert_block_response(agent, &response, &label);
    } else {
        assert!(response.is_none(), "{label}");
    }
}

fn assert_block_response(agent: &str, response: &serde_json::Value, label: &str) {
    if agent == "pi" {
        let block = response.get("block").and_then(serde_json::Value::as_bool);
        assert_eq!(block, Some(true), "{label}");
    } else {
        let decision = response.get("decision").and_then(serde_json::Value::as_str);
        assert_eq!(decision, Some("block"), "{label}");
    }
}

#[test]
fn blocks_plaintext_env_variants_for_file_tools() {
    let expected = Some("Access to sensitive file blocked");
    assert_file_policy(
        &[
            ".env",
            ".env.local",
            ".env.production",
            ".env.example.bak",
            ".env.example.local",
            ".env.example/.env",
            ".env/.env.example",
            "home/../.env.example",
            ".environment",
            "/app/.env.local",
            "/app/.env.local.backup",
            ".env.pub",
            ".env.age.bak",
            ".env.age/.env",
            ".env/.env.age",
            "home/../.env.age",
            ".env.yaml",
            ".env..yaml",
            ".env.prod.yml",
            ".env.prod.yaml.bak",
            ".env.prod.yaml/.env.local",
            ".env/.env.prod.yaml",
            ".env/config.pub",
        ],
        expected,
    );
}

#[test]
fn allows_only_file_exceptions_inside_sensitive_paths() {
    assert_file_policy(
        &[
            ".env.example",
            "/app/.env.example",
            ".env.production.yaml",
            "/app/.env.production.eu.yaml",
            ".env.age",
            "home/.env.age",
            ".envrc",
            "/app/.envrc",
            "~/.ssh/config",
            "/home/user/.ssh/config",
            ".ssh/id_ed25519.pub",
            "/home/user/.ssh/id_rsa.pub",
            "public-key.pub",
        ],
        None,
    );
}

#[test]
fn file_exceptions_do_not_allow_other_sensitive_operands() {
    let expected = Some("Access to sensitive file blocked");
    assert_file_policy(
        &[
            ".ssh",
            ".ssh/id_ed25519",
            ".ssh/id_ed25519.pub.bak",
            ".ssh/config.bak",
            ".ssh/.env.example",
            ".docker/.env.example",
            ".env.example/id_ed25519.pub",
            ".ssh/.env.production.yaml",
            ".ssh/.env.age",
            ".docker/.env.age",
            ".docker/config.json",
            ".docker/config.pub",
            ".docker/.env.production.yaml",
            ".ssh/../id_ed25519.pub",
            "/proc/self/environ",
            "/proc/123/environ",
        ],
        expected,
    );
    let system_reason = Some("Access to system secret file blocked");
    assert_file_policy(&["/etc/shadow", "/etc/shadow/key.pub"], system_reason);
}

#[test]
fn allows_ordinary_files_without_secret_name_false_positives() {
    assert_file_policy(
        &[
            "src/environment.rs",
            "docs/env.local",
            ".github/workflows/config.yaml",
            "docs/ssh/config",
            "config.json",
        ],
        None,
    );
}
