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
fn blocks_cargo_install_path() {
    let output = run_pre_tool_use(&bash("cargo install --path ."));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Use `just install` instead"
    );
}

#[test]
fn allows_cargo_install_registry() {
    let output = run_pre_tool_use(&bash("cargo install ripgrep"));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn blocks_git_c() {
    let output = run_pre_tool_use(&bash("git -C /some/path status"));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Use relative paths instead of `git -C`"
    );
}

#[test]
fn allows_git_status() {
    let output = run_pre_tool_use(&bash("git status"));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn blocks_npm_run() {
    let output = run_pre_tool_use(&bash("npm run build"));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Use `just <task>` instead of `npm run`"
    );
}

#[test]
fn allows_npm_install() {
    let output = run_pre_tool_use(&bash("npm install lodash"));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn blocks_npx_playwright_install() {
    let output = run_pre_tool_use(&bash("npx playwright install"));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Use `just playwright-install` instead"
    );
}

#[test]
fn allows_npx_playwright_test() {
    let output = run_pre_tool_use(&bash("npx playwright test"));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn blocks_npx_run() {
    let output = run_pre_tool_use(&bash("npx run build"));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Use `just <task>` instead of `npx run`"
    );
}

#[test]
fn blocks_skip_hooks() {
    let output = run_pre_tool_use(&bash("SKIP_HOOKS=1 just check"));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Don't skip hooks"
    );
}

#[test]
fn blocks_docker_exec_env() {
    let output = run_pre_tool_use(&bash("docker exec mycontainer env"));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Never read container environment variables"
    );
}

#[test]
fn allows_docker_exec_ls() {
    let output = run_pre_tool_use(&bash("docker exec mycontainer ls"));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn blocks_age_decrypt() {
    let output = run_pre_tool_use(&bash("age --decrypt secret.age"));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to sensitive command blocked"
    );
}

#[test]
fn blocks_sops_decrypt() {
    let output = run_pre_tool_use(&bash("sops --decrypt secrets.yaml"));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Don't decrypt sops files, just read them directly"
    );
}

#[test]
fn blocks_env_file() {
    let output = run_pre_tool_use(&bash("cat .env"));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Never access .env files directly"
    );
}

#[test]
fn blocks_env_in_path() {
    let output = run_pre_tool_use(&bash("source /app/.env"));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Never access .env files directly"
    );
}

#[test]
fn allows_env_example() {
    let output = run_pre_tool_use(&bash("cat .env.example"));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_git_add_env() {
    let output = run_pre_tool_use(&bash("git add .env"));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_git_commit_env_message() {
    let output = run_pre_tool_use(&bash(r#"git commit -m "[feat] Env: update .env""#));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn blocks_gleam_run() {
    let output = run_pre_tool_use(&bash("gleam run"));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Use just run instead"
    );
}

#[test]
fn allows_gleam_build() {
    let output = run_pre_tool_use(&bash("gleam build"));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn blocks_gleam_run_with_redirect() {
    let output = run_pre_tool_use(&bash("gleam run 2>&1"));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Use just run instead"
    );
}

#[test]
fn allows_gleam_run_mid_command() {
    let output = run_pre_tool_use(&bash("echo gleam run"));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn blocks_security_command() {
    let output = run_pre_tool_use(&bash("security find-generic-password"));
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to macOS security command blocked"
    );
}

#[test]
fn blocks_update_snapshots() {
    let output = run_pre_tool_use(&bash("npx playwright test --update-snapshots"));
    assert!(blocked_reason(&output).is_some());
}

#[test]
fn blocks_update_snapshots_underscore() {
    let output = run_pre_tool_use(&bash("pytest --update_snapshots"));
    assert!(blocked_reason(&output).is_some());
}

#[test]
fn blocks_git_filter_branch_all() {
    let output = run_pre_tool_use(&bash(
        "git filter-branch --force --index-filter 'git rm --cached --ignore-unmatch PLAN.md' --prune-empty -- --all",
    ));
    assert!(blocked_reason(&output).is_some());
}

#[test]
fn allows_git_filter_branch_specific() {
    let output = run_pre_tool_use(&bash(
        "git filter-branch --force --index-filter 'git rm --cached --ignore-unmatch PLAN.md' --prune-empty -- main",
    ));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_ls() {
    let output = run_pre_tool_use(&bash("ls -la"));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_just_build() {
    let output = run_pre_tool_use(&bash("just build"));
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_cargo_build() {
    let output = run_pre_tool_use(&bash("cargo build"));
    assert!(blocked_reason(&output).is_none());
}
