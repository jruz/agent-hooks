use std::path;

use regex::Regex;

const SYSTEM_SECRETS: &[&str] = &[
    "/etc/shadow",
    "/etc/passwd",
    "/etc/gshadow",
    "/etc/sudoers",
    "/etc/master.passwd",
];

pub fn run(file_path: &str) -> Result<(), String> {
    let docker = is_docker_path(file_path);
    let allowlisted =
        !docker && (is_envrc(file_path) || is_encrypted_env(file_path) || is_ssh_config(file_path));
    let system_secret = is_system_secret(file_path);
    let sensitive = docker || is_sensitive(file_path);

    if allowlisted {
        Ok(())
    } else if system_secret {
        let reason = "Access to system secret file blocked".to_string();
        Err(reason)
    } else if sensitive {
        let reason = "Access to sensitive file blocked".to_string();
        Err(reason)
    } else {
        Ok(())
    }
}

pub fn is_docker_path(value: &str) -> bool {
    path::Path::new(value)
        .components()
        .any(|component| matches!(component, path::Component::Normal(name) if name == ".docker"))
}

fn is_envrc(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("envrc"))
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
