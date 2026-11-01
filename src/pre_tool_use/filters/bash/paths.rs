use brush_parser::ast;

use super::super::{secrets, shell};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("sed option requires a value")]
    MissingSedOptionValue,
    #[error(transparent)]
    Secrets(#[from] secrets::Error),
    #[error(transparent)]
    Shell(#[from] shell::Error),
}

pub fn run(executable: &str, arguments: &[&ast::Word]) -> Result<(), Error> {
    match executable {
        "command" | "echo" | "env" | "eval" | "exec" | "printf" | "sandbox" | "bash" | "dash"
        | "ksh" | "sh" | "zsh" | "nix-shell" | "docker" | "curl" | "ls" => Ok(()),
        "find" => check_find_paths(arguments),
        "git" => check_git_files(arguments),
        "fd" | "grep" | "rg" => check_search_files(arguments, false),
        "ssh" => check_option_files(arguments, &["-i", "-F", "-E"]),
        "sed" => check_sed_files(arguments, false),
        _ => arguments.iter().try_for_each(|word| check_word(word)),
    }
}

fn check_sed_files(arguments: &[&ast::Word], has_expression: bool) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => check_sed_argument(word, remaining, has_expression),
        None => Ok(()),
    }
}

fn check_sed_argument(
    word: &ast::Word,
    remaining: &[&ast::Word],
    has_expression: bool,
) -> Result<(), Error> {
    let literal = shell::static_word(word)?;
    match literal.as_deref() {
        Some("--") if has_expression => remaining.iter().try_for_each(|path| check_word(path)),
        Some("--") => check_sed_positional(remaining),
        Some("-e" | "--expression" | "-f" | "--file") => {
            let (value, remaining) = remaining
                .split_first()
                .ok_or(Error::MissingSedOptionValue)?;
            if matches!(literal.as_deref(), Some("-e" | "--expression")) {
                check_sed_expression(value)?;
            } else {
                check_word(value)?;
            }
            check_sed_files(remaining, true)
        }
        Some(value) if value.starts_with("--expression=") || value.starts_with("-e") => {
            let expression = value
                .strip_prefix("--expression=")
                .or_else(|| value.strip_prefix("-e"))
                .ok_or(Error::MissingSedOptionValue)?;
            check_sed_expression_text(expression)?;
            check_sed_files(remaining, true)
        }
        Some(value) if value.starts_with("--file=") || value.starts_with("-f") => {
            let file = value
                .strip_prefix("--file=")
                .or_else(|| value.strip_prefix("-f"))
                .ok_or(Error::MissingSedOptionValue)?;
            check_path(file)?;
            check_sed_files(remaining, true)
        }
        Some(value) if value.starts_with('-') => {
            check_word(word)?;
            check_sed_files(remaining, has_expression)
        }
        Some(_) | None if !has_expression => {
            check_sed_expression(word)?;
            check_sed_files(remaining, true)
        }
        Some(_) | None => {
            check_word(word)?;
            check_sed_files(remaining, true)
        }
    }
}

fn check_sed_positional(arguments: &[&ast::Word]) -> Result<(), Error> {
    if let Some((expression, files)) = arguments.split_first() {
        check_sed_expression(expression)?;
        files.iter().try_for_each(|file| check_word(file))?;
    }
    Ok(())
}

fn check_sed_expression(word: &ast::Word) -> Result<(), Error> {
    match shell::static_word(word)? {
        Some(expression) => check_sed_expression_text(&expression),
        None => check_word(word),
    }
}

fn check_sed_expression_text(expression: &str) -> Result<(), Error> {
    if is_literal_substitution(expression) {
        Ok(())
    } else {
        check_path(expression)?;
        expression.split_whitespace().try_for_each(check_path)
    }
}

fn is_literal_substitution(expression: &str) -> bool {
    let Some(substitution) = expression.strip_prefix('s') else {
        return false;
    };
    let mut characters = substitution.chars();
    let Some(delimiter) = characters.next() else {
        return false;
    };
    if delimiter.is_alphanumeric() || delimiter.is_whitespace() || delimiter == '\\' {
        return false;
    }
    let parts = characters.as_str().split(delimiter).collect::<Vec<_>>();
    match parts.as_slice() {
        [pattern, replacement, flags] => {
            !pattern.ends_with('\\')
                && !replacement.ends_with('\\')
                && !pattern.contains('\n')
                && !replacement.contains('\n')
                && flags
                    .chars()
                    .all(|flag| matches!(flag, 'g' | 'p' | 'i' | 'I' | 'm' | 'M' | '0'..='9'))
        }
        _ => false,
    }
}

fn check_git_files(arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => {
            let subcommand = shell::static_word(word)?;
            match subcommand.as_deref() {
                Some("commit") => check_message_file(remaining),
                Some("check-ignore") => Ok(()),
                Some("ls-files") => check_git_listing_files(remaining),
                Some(_) | None => remaining.iter().try_for_each(|path| check_word(path)),
            }
        }
        None => Ok(()),
    }
}

fn check_git_listing_files(arguments: &[&ast::Word]) -> Result<(), Error> {
    let file_options = ["-X", "--exclude-from", "--exclude-per-directory"];
    check_option_files(arguments, &file_options)?;
    arguments
        .iter()
        .try_for_each(|word| check_git_listing_option(word))
}

fn check_git_listing_option(word: &ast::Word) -> Result<(), Error> {
    let literal = brush_parser::unquote_str(&word.value);
    let file = literal
        .strip_prefix("--exclude-from=")
        .or_else(|| literal.strip_prefix("--exclude-per-directory="))
        .or_else(|| literal.strip_prefix("-X"));
    match file {
        Some(file) => check_path(file),
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
    if !value.contains("://") {
        secrets::check_path(value)?;
    }
    Ok(())
}
