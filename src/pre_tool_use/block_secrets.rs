use regex::Regex;

use super::input::ToolInput;

const SYSTEM_SECRETS: &[&str] = &[
    "/etc/shadow",
    "/etc/passwd",
    "/etc/gshadow",
    "/etc/sudoers",
    "/etc/master.passwd",
];

pub fn check(input: &ToolInput) -> Result<(), String> {
    let (ToolInput::Read { file_path } | ToolInput::Grep { file_path }) = input else {
        return Ok(());
    };

    if is_encrypted_env(file_path) {
        return Ok(());
    }

    if is_ssh_config(file_path) {
        return Ok(());
    }

    if is_system_secret(file_path) {
        return Err("Access to system secret file blocked".to_string());
    }

    if is_sensitive(file_path) {
        return Err("Access to sensitive file blocked".to_string());
    }

    Ok(())
}

fn is_encrypted_env(path: &str) -> bool {
    Regex::new(r"\.env\.[^.]+\.yaml$").is_ok_and(|re| re.is_match(path))
}

fn is_ssh_config(path: &str) -> bool {
    path.ends_with(".ssh/config")
}

fn is_system_secret(path: &str) -> bool {
    SYSTEM_SECRETS.iter().any(|secret| path.contains(secret))
}

fn is_sensitive(path: &str) -> bool {
    Regex::new(r"(^|\s|/)(\.env|\.ssh)(\.|/|\s|$)").is_ok_and(|re| re.is_match(path))
}
