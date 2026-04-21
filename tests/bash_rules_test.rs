#[path = "support/pre_tool_use.rs"]
mod helpers;

use std::collections::HashMap;
use std::process::Output;

use helpers::{blocked_reason, payload, run_pre_tool_use};
use serde_json::Value;

fn bash(command: &str) -> helpers::PreToolUsePayload {
    let mut input = HashMap::new();
    let key = "command".to_string();
    let command_value = command.to_string();
    let value = Value::String(command_value);
    input.insert(key, value);
    payload("Bash", input)
}

fn run_bash(command: &str) -> Output {
    let input = bash(command);
    run_pre_tool_use(&input)
}

#[test]
fn blocks_cargo_install_path() {
    let output = run_bash("cargo install --path .");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Use `just install` instead"
    );
}

#[test]
fn blocks_cargo_install_registry() {
    let output = run_bash("cargo install ripgrep");
    assert!(blocked_reason(&output).is_some());
}

#[test]
fn blocks_git_c() {
    let output = run_bash("git -C /some/path status");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Use relative paths instead of `git -C`"
    );
}

#[test]
fn allows_git_status() {
    let output = run_bash("git status");
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn blocks_npm_run() {
    let output = run_bash("npm run build");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Use `just <task>` instead of `npm run`"
    );
}

#[test]
fn blocks_npm_install() {
    let output = run_bash("npm install lodash");
    assert!(blocked_reason(&output).is_some());
}

#[test]
fn blocks_npx_playwright_install() {
    let output = run_bash("npx playwright install");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Use `just playwright-install` instead"
    );
}

#[test]
fn blocks_npx_playwright_test() {
    let output = run_bash("npx playwright test");
    assert!(blocked_reason(&output).is_some());
}

#[test]
fn blocks_npx_run() {
    let output = run_bash("npx run build");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Use `just <task>` instead of `npx run`"
    );
}

#[test]
fn blocks_skip_hooks() {
    let output = run_bash("SKIP_HOOKS=1 just check");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Don't skip hooks"
    );
}

#[test]
fn blocks_docker_exec_env() {
    let output = run_bash("docker exec mycontainer env");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Never read container environment variables"
    );
}

#[test]
fn allows_docker_exec_ls() {
    let output = run_bash("docker exec mycontainer ls");
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn blocks_age_decrypt() {
    let output = run_bash("age --decrypt secret.age");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to sensitive command blocked"
    );
}

#[test]
fn blocks_sops_decrypt() {
    let output = run_bash("sops --decrypt secrets.yaml");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Don't decrypt sops files, just read them directly"
    );
}

#[test]
fn blocks_sops_short_decrypt() {
    let output = run_bash("sops -d secrets.yaml");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Don't decrypt sops files, just read them directly"
    );
}

#[test]
fn blocks_env_file() {
    let output = run_bash("cat .env");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Never access .env files directly"
    );
}

#[test]
fn blocks_env_in_path() {
    let output = run_bash("source /app/.env");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Never access .env files directly"
    );
}

#[test]
fn allows_cat_envrc() {
    let output = run_bash("cat .envrc");
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_env_example() {
    let output = run_bash("cat .env.example");
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_git_add_env() {
    let output = run_bash("git add .env");
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_git_commit_env_message() {
    let output = run_bash(r#"git commit -m "[feat] Env: update .env""#);
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn blocks_gleam_run() {
    let output = run_bash("gleam run");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Use just run instead"
    );
}

#[test]
fn allows_gleam_build() {
    let output = run_bash("gleam build");
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn blocks_gleam_run_with_redirect() {
    let output = run_bash("gleam run 2>&1");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Use just run instead"
    );
}

#[test]
fn allows_gleam_run_mid_command() {
    let output = run_bash("echo gleam run");
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn blocks_security_command() {
    let output = run_bash("security find-generic-password");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to macOS security command blocked"
    );
}

#[test]
fn blocks_update_snapshots() {
    let output = run_bash("npx playwright test --update-snapshots");
    assert!(blocked_reason(&output).is_some());
}

#[test]
fn blocks_update_snapshots_underscore() {
    let output = run_bash("pytest --update_snapshots");
    assert!(blocked_reason(&output).is_some());
}

#[test]
fn blocks_git_filter_branch_all() {
    let output = run_bash(
        "git filter-branch --force --index-filter 'git rm --cached --ignore-unmatch PLAN.md' --prune-empty -- --all",
    );
    assert!(blocked_reason(&output).is_some());
}

#[test]
fn allows_git_filter_branch_specific() {
    let output = run_bash(
        "git filter-branch --force --index-filter 'git rm --cached --ignore-unmatch PLAN.md' --prune-empty -- main",
    );
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_ls() {
    let output = run_bash("ls -la");
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_just_build() {
    let output = run_bash("just build");
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn blocks_bare_cargo() {
    let output = run_bash("cargo build");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Must be prefixed by `sandbox` (e.g. `sandbox cargo build`). Use `just` recipes when available."
    );
}

#[test]
fn blocks_bare_bun() {
    let output = run_bash("bun install");
    assert!(blocked_reason(&output).is_some());
}

#[test]
fn blocks_bare_node() {
    let output = run_bash("node script.js");
    assert!(blocked_reason(&output).is_some());
}

#[test]
fn blocks_bare_python() {
    let output = run_bash("python script.py");
    assert!(blocked_reason(&output).is_some());
}

#[test]
fn blocks_bare_python3() {
    let output = run_bash("python3 script.py");
    assert!(blocked_reason(&output).is_some());
}

#[test]
fn blocks_bare_pnpm() {
    let output = run_bash("pnpm install");
    assert!(blocked_reason(&output).is_some());
}

#[test]
fn blocks_bare_deno() {
    let output = run_bash("deno run script.ts");
    assert!(blocked_reason(&output).is_some());
}

#[test]
fn allows_sandbox_cargo() {
    let output = run_bash("sandbox cargo build");
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn allows_sandbox_bun() {
    let output = run_bash("sandbox bun install");
    assert!(blocked_reason(&output).is_none());
}

#[test]
fn blocks_cat_etc_shadow() {
    let output = run_bash("cat /etc/shadow");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to system secret file blocked"
    );
}

#[test]
fn blocks_cat_etc_passwd() {
    let output = run_bash("cat /etc/passwd");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to system secret file blocked"
    );
}

#[test]
fn blocks_grep_etc_shadow() {
    let output = run_bash("grep root /etc/shadow");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to system secret file blocked"
    );
}

#[test]
fn blocks_ssh_cat_etc_shadow() {
    let output = run_bash("ssh user@host cat /etc/shadow");
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Access to system secret file blocked"
    );
}

#[test]
fn blocks_git_commit_with_co_authored_by() {
    let output = run_bash(
        r#"git commit -m "[feat] Auth: add login" --trailer "Co-Authored-By: Claude <noreply@anthropic.com>""#,
    );
    assert_eq!(
        blocked_reason(&output).expect("should block"),
        "Never include Co-Authored-By in commit messages"
    );
}

#[test]
fn allows_git_commit_without_co_authored_by() {
    let output = run_bash(r#"git commit -m "[feat] Auth: add login""#);
    assert!(blocked_reason(&output).is_none());
}
