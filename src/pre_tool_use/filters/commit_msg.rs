use brush_parser::{ast, word};
use regex::Regex;

use super::shell;

const COMMIT_TITLE_PATTERN: &str = r"^\[(feat|bugs|refa|deps|tool)\] \S+.*: .+";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Never include Co-Authored-By in commit messages")]
    Coauthor,
    #[error("Invalid commit title, follow the Skill(commit) format")]
    InvalidTitle,
    #[error("Could not extract commit title")]
    MissingTitle,
    #[error(transparent)]
    Regex(#[from] regex::Error),
    #[error(transparent)]
    Shell(#[from] shell::Error),
}

pub fn run(arguments: &[&ast::Word]) -> Result<(), Error> {
    let values = arguments
        .iter()
        .map(|word| message_text(word))
        .collect::<Result<Vec<_>, _>>()?;
    if values
        .iter()
        .flatten()
        .any(|value| value.contains("Co-Authored-By"))
    {
        return Err(Error::Coauthor);
    }
    if !should_skip(&values) {
        check_title(&values)?;
    }
    Ok(())
}

fn should_skip(arguments: &[Option<String>]) -> bool {
    match arguments.split_first() {
        Some((Some(argument), remaining)) => match argument.as_str() {
            "--no-edit" | "--fixup" => true,
            argument if argument.starts_with("--fixup=") => true,
            "-m" | "--message" | "--trailer" | "--author" | "-F" | "--file" => {
                match remaining.split_first() {
                    Some((_, remaining)) => should_skip(remaining),
                    None => false,
                }
            }
            _ => should_skip(remaining),
        },
        Some((None, remaining)) => should_skip(remaining),
        None => false,
    }
}

fn check_title(arguments: &[Option<String>]) -> Result<(), Error> {
    let title = extract_title(arguments).ok_or(Error::MissingTitle)?;
    let expression = Regex::new(COMMIT_TITLE_PATTERN)?;
    if expression.is_match(title) {
        Ok(())
    } else {
        Err(Error::InvalidTitle)
    }
}

fn extract_title(arguments: &[Option<String>]) -> Option<&str> {
    match arguments.split_first() {
        Some((Some(argument), remaining)) => match argument.as_str() {
            "-m" | "--message" => remaining.first()?.as_deref()?.lines().next(),
            argument if argument.starts_with("--message=") => {
                argument.strip_prefix("--message=")?.lines().next()
            }
            argument if argument.starts_with("-m") => argument.strip_prefix("-m")?.lines().next(),
            _ => extract_title(remaining),
        },
        Some((None, remaining)) => extract_title(remaining),
        None => None,
    }
}

fn message_text(word: &ast::Word) -> Result<Option<String>, Error> {
    if let Some(text) = shell::static_word(word)? {
        let text = Some(text);
        return Ok(text);
    }
    let pieces = shell::parse_word(&word.value)?;
    heredoc_word(&pieces)
}

fn heredoc_word(pieces: &[word::WordPieceWithSource]) -> Result<Option<String>, Error> {
    match pieces {
        [piece] => match &piece.piece {
            word::WordPiece::DoubleQuotedSequence(nested) => heredoc_word(nested),
            word::WordPiece::CommandSubstitution(source) => heredoc_source(source),
            word::WordPiece::Text(_)
            | word::WordPiece::SingleQuotedText(_)
            | word::WordPiece::AnsiCQuotedText(_)
            | word::WordPiece::GettextDoubleQuotedSequence(_)
            | word::WordPiece::TildeExpansion(_)
            | word::WordPiece::ParameterExpansion(_)
            | word::WordPiece::BackquotedCommandSubstitution(_)
            | word::WordPiece::EscapeSequence(_)
            | word::WordPiece::ArithmeticExpression(_) => Ok(None),
        },
        _ => Ok(None),
    }
}

fn heredoc_source(source: &str) -> Result<Option<String>, Error> {
    let program = shell::parse_program(source)?;
    let [commands] = program.complete_commands.as_slice() else {
        return Ok(None);
    };
    let [item] = commands.0.as_slice() else {
        return Ok(None);
    };
    let pipeline = &item.0.first;
    if !item.0.additional.is_empty()
        || pipeline.bang
        || matches!(item.1, ast::SeparatorOperator::Async)
    {
        return Ok(None);
    }
    match pipeline.seq.as_slice() {
        [ast::Command::Simple(command)] => heredoc_command(command),
        _ => Ok(None),
    }
}

fn heredoc_command(command: &ast::SimpleCommand) -> Result<Option<String>, Error> {
    let Some(word) = command.word_or_name.as_ref() else {
        return Ok(None);
    };
    let executable = shell::static_word(word)?;
    if executable.as_deref() != Some("cat") {
        return Ok(None);
    }
    let suffix = command.suffix.iter().flat_map(|suffix| &suffix.0);
    let items = command
        .prefix
        .iter()
        .flat_map(|prefix| &prefix.0)
        .chain(suffix)
        .collect::<Vec<_>>();
    match items.as_slice() {
        [ast::CommandPrefixOrSuffixItem::IoRedirect(redirect)] => heredoc_redirect(redirect),
        _ => Ok(None),
    }
}

fn heredoc_redirect(redirect: &ast::IoRedirect) -> Result<Option<String>, Error> {
    match redirect {
        ast::IoRedirect::HereDocument(_, document) => {
            let text = shell::heredoc_text(document)?;
            Ok(text)
        }
        ast::IoRedirect::File(_, _, _)
        | ast::IoRedirect::HereString(_, _)
        | ast::IoRedirect::OutputAndError(_, _) => Ok(None),
    }
}
