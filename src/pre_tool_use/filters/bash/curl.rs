use brush_parser::ast;

use super::{super::shell, paths};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Path(#[from] paths::Error),
    #[error(transparent)]
    Shell(#[from] shell::Error),
}

#[derive(Clone, Copy)]
enum OptionKind {
    Data,
    File,
    Literal,
    Url,
}

pub fn run(arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => {
            let literal = shell::static_word(word)?;
            match literal.as_deref() {
                Some(value) => check_argument(value, remaining),
                None => run(remaining),
            }
        }
        None => Ok(()),
    }
}

fn check_argument(value: &str, remaining: &[&ast::Word]) -> Result<(), Error> {
    if let Some((option, value)) = value.split_once('=')
        && let Some(kind) = option_kind(option)
    {
        check_value(kind, value)?;
        return run(remaining);
    }
    if let Some(kind) = option_kind(value) {
        return check_option_value(kind, remaining);
    }
    for option in ["-d", "-F", "-o", "-K", "-T", "-b", "-c", "-E"] {
        if let Some(value) = value.strip_prefix(option)
            && let Some(kind) = option_kind(option)
        {
            check_value(kind, value)?;
            return run(remaining);
        }
    }
    check_file_url(value)?;
    run(remaining)
}

fn option_kind(option: &str) -> Option<OptionKind> {
    match option {
        "-o" | "--output" | "-K" | "--config" | "-T" | "--upload-file" | "-b" | "--cookie"
        | "-c" | "--cookie-jar" | "--cacert" | "--cert" | "-E" | "--key" | "--netrc-file"
        | "--trace" | "--trace-ascii" => Some(OptionKind::File),
        "-d" | "--data" | "--data-ascii" | "--data-binary" | "--data-urlencode" | "--json"
        | "-F" | "--form" => Some(OptionKind::Data),
        "--data-raw" | "--form-string" => Some(OptionKind::Literal),
        "--url" => Some(OptionKind::Url),
        _ => None,
    }
}

fn check_option_value(kind: OptionKind, arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => {
            let literal = shell::static_word(word)?;
            let value = match literal {
                Some(literal) => literal,
                None => brush_parser::unquote_str(&word.value),
            };
            check_value(kind, &value)?;
            run(remaining)
        }
        None => Ok(()),
    }
}

fn check_value(kind: OptionKind, value: &str) -> Result<(), Error> {
    match kind {
        OptionKind::Data => check_data_file(value),
        OptionKind::File => {
            paths::check_path(value)?;
            Ok(())
        }
        OptionKind::Literal => Ok(()),
        OptionKind::Url => check_file_url(value),
    }
}

fn check_data_file(value: &str) -> Result<(), Error> {
    let file = value
        .strip_prefix('@')
        .or_else(|| value.split_once("=@").map(|(_, path)| path));
    if let Some(path) = file {
        let path = path.split(';').next();
        if let Some(path) = path {
            paths::check_path(path)?;
        }
    }
    Ok(())
}

fn check_file_url(value: &str) -> Result<(), Error> {
    if let Some(path) = value.strip_prefix("file://") {
        paths::check_path(path)?;
    }
    Ok(())
}
