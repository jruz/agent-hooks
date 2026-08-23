use std::env;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub fn fixture_contents(relative_path: &str) -> String {
    let path = fixture_path(relative_path);
    fs::read_to_string(path).expect("read fixture")
}

pub fn run_fixture(subcommand: &str, agent: &str, relative_path: &str) -> Output {
    let stdin = fixture_contents(relative_path);
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    run_hook_in_dir(subcommand, agent, &stdin, manifest_dir, &[])
}

pub fn run_hook_in_dir(
    subcommand: &str,
    agent: &str,
    stdin_json: &str,
    current_dir: &Path,
    envs: &[(&str, &str)],
) -> Output {
    let mut tmp = tempfile::NamedTempFile::new().expect("create temp file");
    let stdin_bytes = stdin_json.as_bytes();
    tmp.write_all(stdin_bytes).expect("write temp file");
    let tmp_path = tmp.into_temp_path();

    let stdin_file = std::fs::File::open(tmp_path).expect("open temp file");
    let mut command = Command::new(env!("CARGO_BIN_EXE_agent-hooks"));
    command
        .arg(subcommand)
        .arg("--agent")
        .arg(agent)
        .current_dir(current_dir)
        .stdin(stdin_file);

    for (key, value) in envs {
        command.env(key, value);
    }

    command.output().expect("run binary")
}

pub fn run_hook_in_dir_no_agent(
    subcommand: &str,
    stdin_json: &str,
    current_dir: &Path,
    envs: &[(&str, &str)],
) -> Output {
    let mut tmp = tempfile::NamedTempFile::new().expect("create temp file");
    let stdin_bytes = stdin_json.as_bytes();
    tmp.write_all(stdin_bytes).expect("write temp file");
    let tmp_path = tmp.into_temp_path();

    let stdin_file = std::fs::File::open(tmp_path).expect("open temp file");
    let mut command = Command::new(env!("CARGO_BIN_EXE_agent-hooks"));
    command
        .arg(subcommand)
        .current_dir(current_dir)
        .stdin(stdin_file);

    for (key, value) in envs {
        command.env(key, value);
    }

    command.output().expect("run binary")
}

pub fn blocked_reason(output: &Output) -> Option<String> {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return None;
    }
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("parse stdout as JSON");
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

#[cfg(unix)]
pub fn write_executable(path: &Path, contents: &str) {
    use std::os::unix::fs::PermissionsExt as _;

    fs::write(path, contents).expect("write script");

    let mut perms = fs::metadata(path).expect("stat script").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("chmod script");
}

#[cfg(not(unix))]
pub fn write_executable(path: &Path, contents: &str) {
    fs::write(path, contents).expect("write script");
}

pub fn path_with_bin(bin_dir: &Path) -> String {
    let mut paths = vec![PathBuf::from(bin_dir)];
    if let Some(existing) = env::var_os("PATH") {
        let existing_paths = env::split_paths(&existing);
        paths.extend(existing_paths);
    }

    env::join_paths(paths)
        .expect("join PATH")
        .to_string_lossy()
        .into_owned()
}

fn fixture_path(relative_path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(relative_path)
}
