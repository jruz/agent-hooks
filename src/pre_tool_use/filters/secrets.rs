use std::path;

const SYSTEM_SECRETS: &[&str] = &[
    "/etc/shadow",
    "/etc/passwd",
    "/etc/gshadow",
    "/etc/sudoers",
    "/etc/master.passwd",
];

const HISTORY_FILES: &[&str] = &[
    ".bash_history",
    ".zsh_history",
    ".histfile",
    ".python_history",
    ".node_repl_history",
    ".psql_history",
    ".mysql_history",
    ".sqlite_history",
];

const HISTORY_PATHS: &[&str] = &[
    "nushell/history.txt",
    "nushell/history.sqlite3",
    "fish/fish_history",
];

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Never access .env files directly")]
    Env,
    #[error("Access to sensitive file blocked")]
    Sensitive,
    #[error("Access to system secret file blocked")]
    System,
}

pub fn run(file_path: &str) -> Result<(), String> {
    match check_path(file_path) {
        Ok(()) => Ok(()),
        Err(error) => {
            let reason = match error {
                Error::Env => Error::Sensitive.to_string(),
                Error::Sensitive | Error::System => error.to_string(),
            };
            Err(reason)
        }
    }
}

pub fn check_path(value: &str) -> Result<(), Error> {
    let file = path::Path::new(value);
    let normalized = file.components().collect::<path::PathBuf>();
    if is_system_secret(&normalized) {
        return Err(Error::System);
    }
    if has_directory(file, ".docker") || is_process_environment(file) || is_shell_history(file) {
        return Err(Error::Sensitive);
    }
    let traversal = file
        .components()
        .any(|part| part == path::Component::ParentDir);
    if has_env_component(file) && (traversal || has_env_parent(file) || !is_encrypted_env(file)) {
        return Err(Error::Env);
    }
    if has_directory(file, ".ssh") && (traversal || !is_ssh_exception(file)) {
        return Err(Error::Sensitive);
    }
    Ok(())
}

fn has_directory(file: &path::Path, directory: &str) -> bool {
    file.components()
        .any(|part| matches!(part, path::Component::Normal(name) if name == directory))
}

fn has_env_component(file: &path::Path) -> bool {
    file.components().any(|part| {
        matches!(part, path::Component::Normal(name) if name.to_str().is_some_and(is_env_name))
    })
}

fn is_env_name(name: &str) -> bool {
    name.starts_with(".env") && name != ".envrc"
}

fn has_env_parent(file: &path::Path) -> bool {
    file.parent().is_some_and(has_env_component)
}

fn is_encrypted_env(file: &path::Path) -> bool {
    file.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix(".env."))
        .and_then(|name| name.strip_suffix(".yaml"))
        .is_some_and(|name| name.split('.').all(|part| !part.is_empty()))
}

fn is_ssh_exception(file: &path::Path) -> bool {
    file.ends_with(".ssh/config") || file.extension().is_some_and(|extension| extension == "pub")
}

fn is_system_secret(file: &path::Path) -> bool {
    file.to_str()
        .is_some_and(|value| SYSTEM_SECRETS.iter().any(|secret| value.contains(secret)))
}

fn is_shell_history(file: &path::Path) -> bool {
    let history_file = file
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| HISTORY_FILES.contains(&name));
    let history_path = HISTORY_PATHS.iter().any(|suffix| file.ends_with(suffix));
    let atuin_data = file
        .ancestors()
        .any(|ancestor| ancestor.ends_with("share/atuin"));
    history_file || history_path || atuin_data
}

fn is_process_environment(file: &path::Path) -> bool {
    file.starts_with("/proc") && file.file_name().is_some_and(|name| name == "environ")
}
