use std::path::{Component, Path};

use brush_parser::ast;

use super::super::shell;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Never access .env files directly")]
    Env,
    #[error("Access to sensitive file blocked")]
    Sensitive,
    #[error(transparent)]
    Shell(#[from] shell::Error),
    #[error("Access to system secret file blocked")]
    System,
}

pub fn run(executable: &str, arguments: &[&ast::Word]) -> Result<(), Error> {
    match executable {
        "command" | "echo" | "env" | "eval" | "exec" | "printf" | "sandbox" | "bash" | "dash"
        | "ksh" | "sh" | "zsh" | "nix-shell" | "docker" | "curl" => Ok(()),
        "find" => check_find_paths(arguments),
        "git" => check_git_files(arguments),
        "fd" | "grep" | "rg" => check_search_files(arguments, false),
        "ssh" => check_option_files(arguments, &["-i", "-F", "-E"]),
        _ => arguments.iter().try_for_each(|word| check_word(word)),
    }
}

fn check_git_files(arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => {
            let subcommand = shell::static_word(word)?;
            match subcommand.as_deref() {
                Some("add") => Ok(()),
                Some("commit") => check_message_file(remaining),
                Some(_) | None => remaining.iter().try_for_each(|path| check_word(path)),
            }
        }
        None => Ok(()),
    }
}

fn check_message_file(arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => {
            let literal = shell::static_word(word)?;
            match literal.as_deref() {
                Some("-F" | "--file") => {
                    if let Some((path, _)) = remaining.split_first() {
                        check_word(path)?;
                    }
                }
                Some(argument) if argument.starts_with("--file=") => {
                    let path = argument.trim_start_matches("--file=");
                    check_path(path)?;
                }
                Some(_) | None => {}
            }
            check_message_file(remaining)
        }
        None => Ok(()),
    }
}

fn check_search_files(arguments: &[&ast::Word], has_pattern: bool) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => {
            let literal = shell::static_word(word)?;
            match literal.as_deref() {
                Some("-e" | "--regexp") => check_search_after_option(remaining, true),
                Some("-f" | "--file") => {
                    if let Some((path, _)) = remaining.split_first() {
                        check_word(path)?;
                    }
                    check_search_after_option(remaining, true)
                }
                Some(argument) if argument.starts_with("--file=") => {
                    let path = argument.trim_start_matches("--file=");
                    check_path(path)?;
                    check_search_files(remaining, true)
                }
                Some(argument) if argument.starts_with("--regexp=") => {
                    check_search_files(remaining, true)
                }
                Some("-g" | "--glob" | "-t" | "--type" | "--type-not") => {
                    check_search_after_option(remaining, has_pattern)
                }
                Some(argument) if argument.starts_with('-') => {
                    check_search_files(remaining, has_pattern)
                }
                Some(_) | None if !has_pattern => check_search_files(remaining, true),
                Some(_) | None => {
                    check_word(word)?;
                    check_search_files(remaining, true)
                }
            }
        }
        None => Ok(()),
    }
}

fn check_search_after_option(arguments: &[&ast::Word], has_pattern: bool) -> Result<(), Error> {
    match arguments.split_first() {
        Some((_, remaining)) => check_search_files(remaining, has_pattern),
        None => Ok(()),
    }
}

fn check_option_files(arguments: &[&ast::Word], options: &[&str]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => {
            let literal = shell::static_word(word)?;
            if literal
                .as_deref()
                .is_some_and(|option| options.contains(&option))
            {
                if let Some((path, remaining)) = remaining.split_first() {
                    check_word(path)?;
                    check_option_files(remaining, options)
                } else {
                    Ok(())
                }
            } else {
                check_option_files(remaining, options)
            }
        }
        None => Ok(()),
    }
}

fn check_find_paths(arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => {
            let literal = shell::static_word(word)?;
            if literal
                .as_deref()
                .is_some_and(|value| value.starts_with('-'))
            {
                let file_options = [
                    "-files0-from",
                    "-newer",
                    "-anewer",
                    "-cnewer",
                    "-fprint",
                    "-fprint0",
                    "-fprintf",
                ];
                check_option_files(arguments, &file_options)
            } else {
                check_word(word)?;
                check_find_paths(remaining)
            }
        }
        None => Ok(()),
    }
}

pub fn check_word(word: &ast::Word) -> Result<(), Error> {
    let literal = shell::static_word(word)?;
    let path = match literal {
        Some(literal) => literal,
        None => brush_parser::unquote_str(&word.value),
    };
    check_path(&path)
}

pub fn check_path(value: &str) -> Result<(), Error> {
    if value.contains("://") {
        Ok(())
    } else {
        check_file_path(value)
    }
}

fn check_file_path(value: &str) -> Result<(), Error> {
    let path = Path::new(value);
    let components = path.components().collect::<Vec<_>>();
    let env = components
        .iter()
        .any(|component| matches!(component, Component::Normal(name) if *name == ".env"));
    let ssh = components
        .iter()
        .any(|component| matches!(component, Component::Normal(name) if *name == ".ssh"));
    let system_secret = [
        "/etc/shadow",
        "/etc/passwd",
        "/etc/gshadow",
        "/etc/sudoers",
        "/etc/master.passwd",
    ]
    .iter()
    .any(|secret| path.starts_with(secret));
    if env {
        Err(Error::Env)
    } else if system_secret {
        Err(Error::System)
    } else if ssh && !value.ends_with(".ssh/config") {
        Err(Error::Sensitive)
    } else {
        Ok(())
    }
}
