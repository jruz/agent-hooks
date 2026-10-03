use brush_parser::{ast, word};

use super::super::shell;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Never print environment variables")]
    Dump,
    #[error("Shell option requires a value")]
    MissingOptionValue,
    #[error(transparent)]
    Shell(#[from] shell::Error),
}

pub fn run(executable: &str, arguments: &[&ast::Word]) -> Result<(), Error> {
    match executable {
        "printenv" => Err(Error::Dump),
        "echo" | "printf" | "print" => arguments
            .iter()
            .try_for_each(|word| check_output_word(word)),
        "set" if arguments.is_empty() => Err(Error::Dump),
        "set" | "bash" | "dash" | "ksh" | "sh" | "zsh" => check_trace_arguments(arguments),
        "declare" | "export" | "local" | "readonly" | "typeset" => check_binding_output(arguments),
        "ps" => check_process_output(arguments),
        _ => Ok(()),
    }
}

pub fn check_output(output: shell::Output<'_>) -> Result<(), Error> {
    match output {
        shell::Output::Environment => Err(Error::Dump),
        shell::Output::Expansion(pieces) => check_output_expansion(pieces),
    }
}

fn check_output_expansion(pieces: &[word::WordPieceWithSource]) -> Result<(), Error> {
    if pieces.iter().any(|piece| prints_variable(&piece.piece)) {
        return Err(Error::Dump);
    }
    Ok(())
}

fn check_output_word(word: &ast::Word) -> Result<(), Error> {
    let pieces = shell::parse_word(&word.value)?;
    let output = shell::Output::Expansion(&pieces);
    check_output(output)
}

fn prints_variable(piece: &word::WordPiece) -> bool {
    match piece {
        word::WordPiece::ParameterExpansion(_)
        | word::WordPiece::ArithmeticExpression(_)
        | word::WordPiece::TildeExpansion(_) => true,
        word::WordPiece::DoubleQuotedSequence(pieces)
        | word::WordPiece::GettextDoubleQuotedSequence(pieces) => {
            pieces.iter().any(|nested| prints_variable(&nested.piece))
        }
        word::WordPiece::Text(_)
        | word::WordPiece::SingleQuotedText(_)
        | word::WordPiece::AnsiCQuotedText(_)
        | word::WordPiece::EscapeSequence(_)
        | word::WordPiece::CommandSubstitution(_)
        | word::WordPiece::BackquotedCommandSubstitution(_) => false,
    }
}

fn check_binding_output(arguments: &[&ast::Word]) -> Result<(), Error> {
    arguments
        .iter()
        .try_for_each(|word| check_binding_word(word))?;
    let literals = arguments
        .iter()
        .map(|word| shell::static_word(word))
        .collect::<Result<Vec<_>, _>>()?;
    let only_options = literals.iter().all(|literal| {
        literal
            .as_deref()
            .is_some_and(|value| value.starts_with('-'))
    });
    let print_option = literals.iter().any(|literal| {
        literal
            .as_deref()
            .is_some_and(|value| value.starts_with('-') && value.contains('p'))
    });
    if only_options || print_option {
        return Err(Error::Dump);
    }
    Ok(())
}

fn check_binding_word(word: &ast::Word) -> Result<(), Error> {
    let literal = shell::static_word(word)?;
    if literal.is_none() && !shell::is_env_assignment(word) {
        return Err(Error::Dump);
    }
    Ok(())
}

fn check_process_output(arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => check_process_argument(word, remaining),
        None => Ok(()),
    }
}

fn check_process_argument(word: &ast::Word, remaining: &[&ast::Word]) -> Result<(), Error> {
    let literal = shell::static_word(word)?;
    match literal.as_deref() {
        Some(
            "-p" | "--pid" | "-u" | "--user" | "-U" | "--User" | "-o" | "--format" | "--sort"
            | "-g" | "--group" | "-G" | "--Group" | "-t" | "--tty" | "-C",
        ) => {
            let remaining = remaining_after_option(remaining)?;
            check_process_output(remaining)
        }
        Some(value) if !value.starts_with('-') && value.contains('e') => Err(Error::Dump),
        Some(_) => check_process_output(remaining),
        None => Err(Error::Dump),
    }
}

fn check_trace_arguments(arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => check_trace_argument(word, remaining),
        None => Ok(()),
    }
}

fn check_trace_argument(word: &ast::Word, remaining: &[&ast::Word]) -> Result<(), Error> {
    let literal = shell::static_word(word)?;
    match literal.as_deref() {
        Some("-o") => check_trace_option(remaining),
        Some(value)
            if value.starts_with('-') && !value.starts_with("--") && value.contains('x') =>
        {
            Err(Error::Dump)
        }
        Some("-c" | "--command") => Ok(()),
        Some(value)
            if value.starts_with('-') && !value.starts_with("--") && value.contains('c') =>
        {
            Ok(())
        }
        Some(_) | None => check_trace_arguments(remaining),
    }
}

fn check_trace_option(arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => {
            let literal = shell::static_word(word)?;
            if literal.as_deref() == Some("xtrace") {
                return Err(Error::Dump);
            }
            check_trace_arguments(remaining)
        }
        None => Ok(()),
    }
}

fn remaining_after_option<'words>(
    arguments: &'words [&ast::Word],
) -> Result<&'words [&'words ast::Word], Error> {
    let (_, remaining) = arguments.split_first().ok_or(Error::MissingOptionValue)?;
    Ok(remaining)
}
