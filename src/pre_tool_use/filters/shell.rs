use std::io;

use brush_parser::{self, ast, word};

pub type InvocationCheck = fn(&str, &[&ast::Word], Sandbox, Execution) -> Result<(), String>;

#[derive(Clone, Copy)]
pub struct Visitor {
    pub assignment: fn(&ast::Assignment) -> Result<(), String>,
    pub invocation: InvocationCheck,
    pub path: fn(&ast::Word) -> Result<(), String>,
}

#[derive(Clone, Copy)]
struct Context {
    execution: Execution,
    sandbox: Sandbox,
    visitor: Visitor,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Execution {
    Container,
    Local,
    Remote,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Sandbox {
    Inside,
    Outside,
}

#[derive(Debug, Clone, Copy)]
enum Wrapper {
    Command,
    Docker,
    Env,
    Eval,
    Exec,
    Find,
    NixShell,
    Sandbox,
    Shell,
    Ssh,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Dynamic executable names must be invoked through `sandbox`")]
    DynamicExecutable,
    #[error("find execution clauses must end with `;` or `+`")]
    MissingFindTerminator,
    #[error(transparent)]
    Parse(#[from] brush_parser::ParseError),
    #[error("{0}")]
    Policy(String),
    #[error(transparent)]
    WordParse(#[from] brush_parser::WordParseError),
}

pub fn run(command: &str, visitor: &Visitor) -> Result<(), Error> {
    let program = parse_program(command)?;
    let context = Context {
        execution: Execution::Local,
        sandbox: Sandbox::Outside,
        visitor: *visitor,
    };
    check_program(&context, &program)
}

fn policy_result(result: Result<(), String>) -> Result<(), Error> {
    match result {
        Ok(()) => Ok(()),
        Err(reason) => {
            let error = Error::Policy(reason);
            Err(error)
        }
    }
}

fn parser_options() -> brush_parser::ParserOptions {
    brush_parser::ParserOptions {
        enable_extended_globbing: true,
        posix_mode: false,
        sh_mode: false,
        tilde_expansion_at_word_start: true,
        tilde_expansion_after_colon: false,
        parser_impl: brush_parser::ParserImpl::Peg,
    }
}

pub fn parse_program(command: &str) -> Result<ast::Program, Error> {
    let bytes = command.as_bytes();
    let reader = io::Cursor::new(bytes);
    let options = parser_options();
    let mut parser = brush_parser::Parser::new(reader, &options);
    let program = parser.parse_program()?;
    Ok(program)
}

fn check_program(context: &Context, program: &ast::Program) -> Result<(), Error> {
    program
        .complete_commands
        .iter()
        .try_for_each(|value| check_compound_list(context, value))
}

fn check_compound_list(context: &Context, list: &ast::CompoundList) -> Result<(), Error> {
    list.0
        .iter()
        .try_for_each(|item| check_and_or_list(context, &item.0))
}

fn check_and_or_list(context: &Context, list: &ast::AndOrList) -> Result<(), Error> {
    check_pipeline(context, &list.first)?;
    list.additional.iter().try_for_each(|item| match item {
        ast::AndOr::And(pipeline) | ast::AndOr::Or(pipeline) => check_pipeline(context, pipeline),
    })
}

fn check_pipeline(context: &Context, pipeline: &ast::Pipeline) -> Result<(), Error> {
    pipeline
        .seq
        .iter()
        .try_for_each(|value| check_command(context, value))
}

fn check_command(context: &Context, command: &ast::Command) -> Result<(), Error> {
    match command {
        ast::Command::Simple(command) => check_simple_command(context, command),
        ast::Command::Compound(command, redirects) => {
            check_compound_command(context, command)?;
            let redirects = redirects.as_ref();
            check_optional_redirects(context, redirects)
        }
        ast::Command::Function(function) => check_compound_command(context, &function.body.0),
        ast::Command::ExtendedTest(test, redirects) => {
            check_extended_test(context, &test.expr)?;
            let redirects = redirects.as_ref();
            check_optional_redirects(context, redirects)
        }
    }
}

fn check_simple_command(context: &Context, command: &ast::SimpleCommand) -> Result<(), Error> {
    let prefix = command.prefix.as_ref();
    let prefix = prefix_items(prefix);
    check_optional_items(context, prefix)?;

    let suffix = command.suffix.as_ref();
    let suffix_items = suffix_items(suffix);
    check_suffix_items(context, suffix_items)?;
    let arguments = command_words(suffix);

    match command.word_or_name.as_ref() {
        Some(word) => {
            check_word(context, word)?;
            check_executable_word(context, word, &arguments)
        }
        None => Ok(()),
    }
}

fn command_words(suffix: Option<&ast::CommandSuffix>) -> Vec<&ast::Word> {
    suffix
        .into_iter()
        .flat_map(|suffix| &suffix.0)
        .filter_map(|item| match item {
            ast::CommandPrefixOrSuffixItem::Word(word)
            | ast::CommandPrefixOrSuffixItem::AssignmentWord(_, word) => Some(word),
            ast::CommandPrefixOrSuffixItem::IoRedirect(_)
            | ast::CommandPrefixOrSuffixItem::ProcessSubstitution(_, _) => None,
        })
        .collect()
}

fn prefix_items(prefix: Option<&ast::CommandPrefix>) -> Option<&[ast::CommandPrefixOrSuffixItem]> {
    match prefix {
        Some(prefix) => {
            let items = prefix.0.as_slice();
            Some(items)
        }
        None => None,
    }
}

fn suffix_items(suffix: Option<&ast::CommandSuffix>) -> Option<&[ast::CommandPrefixOrSuffixItem]> {
    match suffix {
        Some(suffix) => {
            let items = suffix.0.as_slice();
            Some(items)
        }
        None => None,
    }
}

fn check_executable_word(
    context: &Context,
    word: &ast::Word,
    arguments: &[&ast::Word],
) -> Result<(), Error> {
    let executable = static_word(word)?;
    match executable {
        Some(executable) => check_invocation(context, &executable, arguments),
        None if context.sandbox == Sandbox::Inside => Ok(()),
        None => Err(Error::DynamicExecutable),
    }
}

fn check_invocation(
    context: &Context,
    executable: &str,
    arguments: &[&ast::Word],
) -> Result<(), Error> {
    let result =
        (context.visitor.invocation)(executable, arguments, context.sandbox, context.execution);
    policy_result(result)?;
    let name = executable_name(executable);
    let wrapper = if executable == "sandbox" {
        Some(Wrapper::Sandbox)
    } else {
        wrapper(name)
    };
    match wrapper {
        Some(wrapper) => check_wrapper(context, wrapper, arguments),
        None => Ok(()),
    }
}

fn wrapper(name: &str) -> Option<Wrapper> {
    match name {
        "command" => Some(Wrapper::Command),
        "docker" => Some(Wrapper::Docker),
        "env" => Some(Wrapper::Env),
        "eval" => Some(Wrapper::Eval),
        "exec" => Some(Wrapper::Exec),
        "find" => Some(Wrapper::Find),
        "nix-shell" => Some(Wrapper::NixShell),
        "bash" | "dash" | "ksh" | "sh" | "zsh" => Some(Wrapper::Shell),
        "ssh" => Some(Wrapper::Ssh),
        _ => None,
    }
}

fn check_wrapper(
    context: &Context,
    wrapper: Wrapper,
    arguments: &[&ast::Word],
) -> Result<(), Error> {
    match wrapper {
        Wrapper::Command => check_command_wrapper(context, arguments),
        Wrapper::Docker => check_docker(context, arguments),
        Wrapper::Env => check_env(context, arguments),
        Wrapper::Eval => check_eval(context, arguments),
        Wrapper::Exec => check_exec(context, arguments),
        Wrapper::Find => check_find(context, arguments),
        Wrapper::NixShell => check_nix_shell(context, arguments),
        Wrapper::Sandbox => {
            let nested = Context {
                sandbox: Sandbox::Inside,
                ..*context
            };
            check_launcher(&nested, arguments)
        }
        Wrapper::Shell => check_shell(context, arguments),
        Wrapper::Ssh => check_ssh(context, arguments),
    }
}

fn check_docker(context: &Context, arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => {
            let subcommand = static_word(word)?;
            if subcommand.as_deref() == Some("exec") {
                let options = [
                    "-e",
                    "--env",
                    "-u",
                    "--user",
                    "-w",
                    "--workdir",
                    "--detach-keys",
                ];
                let launched = launched_arguments(remaining, &options)?;
                let nested = Context {
                    execution: Execution::Container,
                    sandbox: Sandbox::Outside,
                    visitor: context.visitor,
                };
                check_launcher(&nested, launched)
            } else {
                Ok(())
            }
        }
        None => Ok(()),
    }
}

fn check_ssh(context: &Context, arguments: &[&ast::Word]) -> Result<(), Error> {
    let options = [
        "-b", "-c", "-D", "-E", "-e", "-F", "-i", "-J", "-L", "-l", "-m", "-o", "-p", "-R", "-S",
        "-W", "-w",
    ];
    let arguments = launched_arguments(arguments, &options)?;
    let nested = Context {
        execution: Execution::Remote,
        sandbox: Sandbox::Outside,
        visitor: context.visitor,
    };
    check_eval(&nested, arguments)
}

fn launched_arguments<'words>(
    arguments: &'words [&ast::Word],
    value_options: &[&str],
) -> Result<&'words [&'words ast::Word], Error> {
    match arguments.split_first() {
        Some((word, remaining)) => {
            let literal = static_word(word)?;
            match literal.as_deref() {
                Some(option) if value_options.contains(&option) => match remaining.split_first() {
                    Some((_, remaining)) => launched_arguments(remaining, value_options),
                    None => Ok(remaining),
                },
                Some("--") => match remaining.split_first() {
                    Some((_, remaining)) => Ok(remaining),
                    None => Ok(remaining),
                },
                Some(option) if option.starts_with('-') => {
                    launched_arguments(remaining, value_options)
                }
                Some(_) | None => Ok(remaining),
            }
        }
        None => Ok(arguments),
    }
}

fn check_launcher(context: &Context, arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((executable, remaining)) => check_executable_word(context, executable, remaining),
        None => Ok(()),
    }
}

fn check_shell(context: &Context, arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => {
            let literal = static_word(word)?;
            match literal.as_deref() {
                Some("-c" | "--command") => check_source_argument(context, remaining),
                Some(argument)
                    if argument.starts_with('-')
                        && !argument.starts_with("--")
                        && argument.contains('c') =>
                {
                    check_source_argument(context, remaining)
                }
                Some(_) | None => check_shell(context, remaining),
            }
        }
        None => Ok(()),
    }
}

fn check_eval(context: &Context, arguments: &[&ast::Word]) -> Result<(), Error> {
    let command = arguments
        .iter()
        .map(|word| literal_word(word))
        .collect::<Result<Vec<_>, _>>()?;
    let command = command.join(" ");
    let program = parse_program(&command)?;
    check_program(context, &program)
}

fn check_find(context: &Context, arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => check_find_argument(context, word, remaining),
        None => Ok(()),
    }
}

fn check_find_argument(
    context: &Context,
    word: &ast::Word,
    remaining: &[&ast::Word],
) -> Result<(), Error> {
    let literal = static_word(word)?;
    if literal.as_deref().is_some_and(is_find_launcher) {
        check_find_launcher(context, remaining)
    } else if literal.as_deref().is_some_and(is_find_value_option) {
        match remaining.split_first() {
            Some((_, remaining)) => check_find(context, remaining),
            None => Ok(()),
        }
    } else {
        check_find(context, remaining)
    }
}

fn check_find_launcher(context: &Context, arguments: &[&ast::Word]) -> Result<(), Error> {
    let literals = arguments
        .iter()
        .map(|word| static_word(word))
        .collect::<Result<Vec<_>, _>>()?;
    let end = literals
        .iter()
        .position(|literal| matches!(literal.as_deref(), Some(";" | "+")));
    let end = end.ok_or(Error::MissingFindTerminator)?;
    let (launched, remaining) = arguments.split_at(end);
    check_launcher(context, launched)?;
    let remaining = remaining.get(1..).ok_or(Error::MissingFindTerminator)?;
    check_find(context, remaining)
}

fn is_find_value_option(argument: &str) -> bool {
    matches!(
        argument,
        "-name"
            | "-iname"
            | "-path"
            | "-ipath"
            | "-regex"
            | "-iregex"
            | "-printf"
            | "-fprint"
            | "-fprint0"
            | "-newer"
            | "-anewer"
            | "-cnewer"
            | "-files0-from"
            | "-type"
            | "-user"
            | "-group"
            | "-size"
            | "-mtime"
            | "-mmin"
            | "-maxdepth"
            | "-mindepth"
    )
}

fn is_find_launcher(argument: &str) -> bool {
    matches!(argument, "-exec" | "-execdir" | "-ok" | "-okdir")
}

fn check_nix_shell(context: &Context, arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => {
            let literal = static_word(word)?;
            match literal.as_deref() {
                Some("--run" | "--command") => check_source_argument(context, remaining),
                Some(argument) if argument.starts_with("--run=") => {
                    check_prefixed_source(context, argument, "--run=")
                }
                Some(argument) if argument.starts_with("--command=") => {
                    check_prefixed_source(context, argument, "--command=")
                }
                Some(_) | None => check_nix_shell(context, remaining),
            }
        }
        None => Ok(()),
    }
}

fn check_source_argument(context: &Context, arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, _)) => {
            let source = literal_word(word)?;
            check_source(context, &source)
        }
        None => Ok(()),
    }
}

fn check_prefixed_source(context: &Context, argument: &str, prefix: &str) -> Result<(), Error> {
    match argument.strip_prefix(prefix) {
        Some(source) => check_source(context, source),
        None => Ok(()),
    }
}

fn check_source(context: &Context, source: &str) -> Result<(), Error> {
    let program = parse_program(source)?;
    check_program(context, &program)
}

fn check_env(context: &Context, arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => check_env_argument(context, word, remaining),
        None => Ok(()),
    }
}

fn check_env_argument(
    context: &Context,
    word: &ast::Word,
    remaining: &[&ast::Word],
) -> Result<(), Error> {
    let literal = static_word(word)?;
    match literal.as_deref() {
        Some("-S" | "--split-string") => check_source_argument(context, remaining),
        Some(argument) if argument.starts_with("--split-string=") => {
            check_prefixed_source(context, argument, "--split-string=")
        }
        Some(argument) if argument.starts_with("-S") => {
            check_prefixed_source(context, argument, "-S")
        }
        Some("-a" | "--argv0" | "-u" | "--unset" | "-C" | "--chdir") => {
            check_env_after_option(context, remaining)
        }
        Some(argument) if argument.starts_with('-') || argument.contains('=') => {
            check_env(context, remaining)
        }
        Some(_) => check_executable_word(context, word, remaining),
        None if is_env_assignment(word) => check_env(context, remaining),
        None => Err(Error::DynamicExecutable),
    }
}

fn check_env_after_option(context: &Context, arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((_, remaining)) => check_env(context, remaining),
        None => Ok(()),
    }
}

fn is_env_assignment(word: &ast::Word) -> bool {
    let Some((name, _)) = word.value.split_once('=') else {
        return false;
    };
    !name.is_empty()
        && name
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn check_command_wrapper(context: &Context, arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => {
            let literal = literal_word(word)?;
            match literal.as_str() {
                "-p" | "--" => check_command_wrapper(context, remaining),
                "-v" | "-V" => Ok(()),
                _ => check_executable_word(context, word, remaining),
            }
        }
        None => Ok(()),
    }
}

fn check_exec(context: &Context, arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => {
            let literal = literal_word(word)?;
            match literal.as_str() {
                "-a" => check_exec_after_option(context, remaining),
                argument if argument.starts_with('-') => check_exec(context, remaining),
                _ => check_executable_word(context, word, remaining),
            }
        }
        None => Ok(()),
    }
}

fn check_exec_after_option(context: &Context, arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((_, remaining)) => check_exec(context, remaining),
        None => Ok(()),
    }
}

fn literal_word(word: &ast::Word) -> Result<String, Error> {
    let Some(literal) = static_word(word)? else {
        return Err(Error::DynamicExecutable);
    };
    Ok(literal)
}

pub fn static_word(word: &ast::Word) -> Result<Option<String>, Error> {
    let pieces = parse_word(&word.value)?;
    let options = parser_options();
    let brace_expansions = word::parse_brace_expansions(&word.value, &options)?;
    let brace_expansion_parts = brace_expansions.as_deref();
    let has_brace_expansion = has_brace_expansion(brace_expansion_parts);
    let is_static = pieces.iter().all(|piece| is_static_piece(&piece.piece));
    if has_brace_expansion || !is_static {
        return Ok(None);
    }

    let literal = brush_parser::unquote_str(&word.value);
    let literal = Some(literal);
    Ok(literal)
}

fn has_brace_expansion(parts: Option<&[word::BraceExpressionOrText]>) -> bool {
    match parts {
        Some(parts) => parts
            .iter()
            .any(|part| matches!(part, word::BraceExpressionOrText::Expr(_))),
        None => false,
    }
}

pub fn heredoc_text(document: &ast::IoHereDocument) -> Result<Option<String>, Error> {
    if document.requires_expansion {
        let options = parser_options();
        let pieces = word::parse_heredoc(&document.doc.value, &options)?;
        let is_literal = pieces
            .iter()
            .all(|piece| matches!(piece.piece, word::WordPiece::Text(_)));
        if !is_literal {
            return Ok(None);
        }
    }
    let text = document.doc.value.trim_end_matches('\n').to_string();
    let text = Some(text);
    Ok(text)
}

pub fn parse_word(value: &str) -> Result<Vec<word::WordPieceWithSource>, Error> {
    let options = parser_options();
    let pieces = word::parse(value, &options)?;
    Ok(pieces)
}

fn is_static_piece(piece: &word::WordPiece) -> bool {
    match piece {
        word::WordPiece::Text(text) => !text.contains(['*', '?', '[']),
        word::WordPiece::SingleQuotedText(_) | word::WordPiece::EscapeSequence(_) => true,
        word::WordPiece::DoubleQuotedSequence(pieces) => pieces
            .iter()
            .all(|nested_piece| is_static_quoted_piece(&nested_piece.piece)),
        word::WordPiece::AnsiCQuotedText(_)
        | word::WordPiece::GettextDoubleQuotedSequence(_)
        | word::WordPiece::TildeExpansion(_)
        | word::WordPiece::ParameterExpansion(_)
        | word::WordPiece::CommandSubstitution(_)
        | word::WordPiece::BackquotedCommandSubstitution(_)
        | word::WordPiece::ArithmeticExpression(_) => false,
    }
}

fn is_static_quoted_piece(piece: &word::WordPiece) -> bool {
    matches!(piece, word::WordPiece::Text(_)) || is_static_piece(piece)
}

fn check_word(context: &Context, word: &ast::Word) -> Result<(), Error> {
    let pieces = parse_word(&word.value)?;
    check_word_pieces(context, &pieces)
}

fn check_word_pieces(context: &Context, pieces: &[word::WordPieceWithSource]) -> Result<(), Error> {
    pieces
        .iter()
        .try_for_each(|piece| check_word_piece(context, &piece.piece))
}

fn check_word_piece(context: &Context, piece: &word::WordPiece) -> Result<(), Error> {
    match piece {
        word::WordPiece::DoubleQuotedSequence(pieces)
        | word::WordPiece::GettextDoubleQuotedSequence(pieces) => {
            check_word_pieces(context, pieces)
        }
        word::WordPiece::CommandSubstitution(command)
        | word::WordPiece::BackquotedCommandSubstitution(command) => {
            let program = parse_program(command)?;
            check_program(context, &program)
        }
        word::WordPiece::ParameterExpansion(expression) => {
            check_parameter_expression(context, expression)
        }
        word::WordPiece::ArithmeticExpression(expression) => {
            check_embedded_word(context, &expression.value)
        }
        word::WordPiece::Text(_)
        | word::WordPiece::SingleQuotedText(_)
        | word::WordPiece::AnsiCQuotedText(_)
        | word::WordPiece::TildeExpansion(_)
        | word::WordPiece::EscapeSequence(_) => Ok(()),
    }
}

fn check_parameter_expression(
    context: &Context,
    expression: &word::ParameterExpr,
) -> Result<(), Error> {
    check_parameter(context, expression)?;
    match expression {
        word::ParameterExpr::UseDefaultValues { default_value, .. }
        | word::ParameterExpr::AssignDefaultValues { default_value, .. } => {
            let default_value = default_value.as_deref();
            check_optional_embedded_word(context, default_value)
        }
        word::ParameterExpr::IndicateErrorIfNullOrUnset { error_message, .. } => {
            let error_message = error_message.as_deref();
            check_optional_embedded_word(context, error_message)
        }
        word::ParameterExpr::UseAlternativeValue {
            alternative_value, ..
        } => {
            let alternative_value = alternative_value.as_deref();
            check_optional_embedded_word(context, alternative_value)
        }
        word::ParameterExpr::RemoveSmallestSuffixPattern { pattern, .. }
        | word::ParameterExpr::RemoveLargestSuffixPattern { pattern, .. }
        | word::ParameterExpr::RemoveSmallestPrefixPattern { pattern, .. }
        | word::ParameterExpr::RemoveLargestPrefixPattern { pattern, .. }
        | word::ParameterExpr::UppercaseFirstChar { pattern, .. }
        | word::ParameterExpr::UppercasePattern { pattern, .. }
        | word::ParameterExpr::LowercaseFirstChar { pattern, .. }
        | word::ParameterExpr::LowercasePattern { pattern, .. } => {
            let pattern = pattern.as_deref();
            check_optional_embedded_word(context, pattern)
        }
        word::ParameterExpr::Substring { offset, length, .. } => {
            check_embedded_word(context, &offset.value)?;
            match length {
                Some(length) => check_embedded_word(context, &length.value),
                None => Ok(()),
            }
        }
        word::ParameterExpr::ReplaceSubstring {
            pattern,
            replacement,
            ..
        } => {
            check_embedded_word(context, pattern)?;
            let replacement = replacement.as_deref();
            check_optional_embedded_word(context, replacement)
        }
        word::ParameterExpr::Parameter { .. }
        | word::ParameterExpr::ParameterLength { .. }
        | word::ParameterExpr::Transform { .. }
        | word::ParameterExpr::VariableNames { .. }
        | word::ParameterExpr::MemberKeys { .. } => Ok(()),
    }
}

fn check_parameter(context: &Context, expression: &word::ParameterExpr) -> Result<(), Error> {
    let parameter = match expression {
        word::ParameterExpr::Parameter { parameter, .. }
        | word::ParameterExpr::UseDefaultValues { parameter, .. }
        | word::ParameterExpr::AssignDefaultValues { parameter, .. }
        | word::ParameterExpr::IndicateErrorIfNullOrUnset { parameter, .. }
        | word::ParameterExpr::UseAlternativeValue { parameter, .. }
        | word::ParameterExpr::ParameterLength { parameter, .. }
        | word::ParameterExpr::RemoveSmallestSuffixPattern { parameter, .. }
        | word::ParameterExpr::RemoveLargestSuffixPattern { parameter, .. }
        | word::ParameterExpr::RemoveSmallestPrefixPattern { parameter, .. }
        | word::ParameterExpr::RemoveLargestPrefixPattern { parameter, .. }
        | word::ParameterExpr::Substring { parameter, .. }
        | word::ParameterExpr::Transform { parameter, .. }
        | word::ParameterExpr::UppercaseFirstChar { parameter, .. }
        | word::ParameterExpr::UppercasePattern { parameter, .. }
        | word::ParameterExpr::LowercaseFirstChar { parameter, .. }
        | word::ParameterExpr::LowercasePattern { parameter, .. }
        | word::ParameterExpr::ReplaceSubstring { parameter, .. } => Some(parameter),
        word::ParameterExpr::VariableNames { .. } | word::ParameterExpr::MemberKeys { .. } => None,
    };

    match parameter {
        Some(word::Parameter::NamedWithIndex { index, .. }) => check_embedded_word(context, index),
        Some(
            word::Parameter::Positional(_)
            | word::Parameter::Special(_)
            | word::Parameter::Named(_)
            | word::Parameter::NamedWithAllIndices { .. },
        )
        | None => Ok(()),
    }
}

fn check_optional_embedded_word(context: &Context, value: Option<&str>) -> Result<(), Error> {
    match value {
        Some(value) => check_embedded_word(context, value),
        None => Ok(()),
    }
}

fn check_embedded_word(context: &Context, value: &str) -> Result<(), Error> {
    let pieces = parse_word(value)?;
    check_word_pieces(context, &pieces)
}

fn check_optional_items(
    context: &Context,
    items: Option<&[ast::CommandPrefixOrSuffixItem]>,
) -> Result<(), Error> {
    match items {
        Some(items) => items
            .iter()
            .try_for_each(|value| check_item(context, value)),
        None => Ok(()),
    }
}

fn check_suffix_items(
    context: &Context,
    items: Option<&[ast::CommandPrefixOrSuffixItem]>,
) -> Result<(), Error> {
    match items {
        Some(items) => items.iter().try_for_each(|item| match item {
            ast::CommandPrefixOrSuffixItem::AssignmentWord(_, word) => check_word(context, word),
            ast::CommandPrefixOrSuffixItem::Word(_)
            | ast::CommandPrefixOrSuffixItem::IoRedirect(_)
            | ast::CommandPrefixOrSuffixItem::ProcessSubstitution(_, _) => {
                check_item(context, item)
            }
        }),
        None => Ok(()),
    }
}

fn check_item(context: &Context, item: &ast::CommandPrefixOrSuffixItem) -> Result<(), Error> {
    match item {
        ast::CommandPrefixOrSuffixItem::IoRedirect(redirect) => check_redirect(context, redirect),
        ast::CommandPrefixOrSuffixItem::Word(word) => check_word(context, word),
        ast::CommandPrefixOrSuffixItem::AssignmentWord(assignment, word) => {
            check_assignment(context, assignment)?;
            check_word(context, word)
        }
        ast::CommandPrefixOrSuffixItem::ProcessSubstitution(_, command) => {
            check_compound_list(context, &command.list)
        }
    }
}

fn check_assignment(context: &Context, assignment: &ast::Assignment) -> Result<(), Error> {
    let result = (context.visitor.assignment)(assignment);
    policy_result(result)?;
    match &assignment.value {
        ast::AssignmentValue::Scalar(word) => check_word(context, word),
        ast::AssignmentValue::Array(values) => values.iter().try_for_each(|(key, value)| {
            if let Some(key) = key {
                check_word(context, key)?;
            }
            check_word(context, value)
        }),
    }
}

fn check_optional_redirects(
    context: &Context,
    redirects: Option<&ast::RedirectList>,
) -> Result<(), Error> {
    match redirects {
        Some(redirects) => redirects
            .0
            .iter()
            .try_for_each(|value| check_redirect(context, value)),
        None => Ok(()),
    }
}

fn check_redirect(context: &Context, redirect: &ast::IoRedirect) -> Result<(), Error> {
    match redirect {
        ast::IoRedirect::File(_, _, target) => check_redirect_target(context, target),
        ast::IoRedirect::HereDocument(_, document) if document.requires_expansion => {
            let value = &document.doc.value;
            let options = parser_options();
            let pieces = match word::parse_heredoc(value, &options) {
                Ok(pieces) => pieces,
                Err(error) => {
                    let error = Error::WordParse(error);
                    return Err(error);
                }
            };
            check_word_pieces(context, &pieces)
        }
        ast::IoRedirect::HereDocument(_, _) => Ok(()),
        ast::IoRedirect::HereString(_, word) => check_word(context, word),
        ast::IoRedirect::OutputAndError(word, _) => check_path(context, word),
    }
}

fn check_redirect_target(
    context: &Context,
    target: &ast::IoFileRedirectTarget,
) -> Result<(), Error> {
    match target {
        ast::IoFileRedirectTarget::Filename(word) => check_path(context, word),
        ast::IoFileRedirectTarget::Duplicate(word) => check_word(context, word),
        ast::IoFileRedirectTarget::Fd(_) => Ok(()),
        ast::IoFileRedirectTarget::ProcessSubstitution(_, command) => {
            check_compound_list(context, &command.list)
        }
    }
}

fn check_compound_command(context: &Context, command: &ast::CompoundCommand) -> Result<(), Error> {
    match command {
        ast::CompoundCommand::Arithmetic(command) => {
            check_embedded_word(context, &command.expr.value)
        }
        ast::CompoundCommand::ArithmeticForClause(command) => {
            let initializer = command.initializer.as_ref();
            check_optional_arithmetic(context, initializer)?;
            let condition = command.condition.as_ref();
            check_optional_arithmetic(context, condition)?;
            let updater = command.updater.as_ref();
            check_optional_arithmetic(context, updater)?;
            check_compound_list(context, &command.body.list)
        }
        ast::CompoundCommand::BraceGroup(command) => check_compound_list(context, &command.list),
        ast::CompoundCommand::Subshell(command) => check_compound_list(context, &command.list),
        ast::CompoundCommand::ForClause(command) => {
            if let Some(values) = command.values.as_ref() {
                values
                    .iter()
                    .try_for_each(|value| check_word(context, value))?;
            }
            check_compound_list(context, &command.body.list)
        }
        ast::CompoundCommand::CaseClause(command) => {
            check_word(context, &command.value)?;
            command.cases.iter().try_for_each(|case| {
                case.patterns
                    .iter()
                    .try_for_each(|value| check_word(context, value))?;
                match case.cmd.as_ref() {
                    Some(case_command) => check_compound_list(context, case_command),
                    None => Ok(()),
                }
            })
        }
        ast::CompoundCommand::IfClause(command) => {
            check_compound_list(context, &command.condition)?;
            check_compound_list(context, &command.then)?;
            if let Some(elses) = command.elses.as_ref() {
                elses.iter().try_for_each(|clause| {
                    if let Some(condition) = clause.condition.as_ref() {
                        check_compound_list(context, condition)?;
                    }
                    check_compound_list(context, &clause.body)
                })?;
            }
            Ok(())
        }
        ast::CompoundCommand::WhileClause(command) | ast::CompoundCommand::UntilClause(command) => {
            check_compound_list(context, &command.0)?;
            check_compound_list(context, &command.1.list)
        }
        ast::CompoundCommand::Coprocess(command) => check_command(context, &command.body),
    }
}

fn check_optional_arithmetic(
    context: &Context,
    expression: Option<&ast::UnexpandedArithmeticExpr>,
) -> Result<(), Error> {
    match expression {
        Some(expression) => check_embedded_word(context, &expression.value),
        None => Ok(()),
    }
}

fn check_extended_test(context: &Context, expression: &ast::ExtendedTestExpr) -> Result<(), Error> {
    match expression {
        ast::ExtendedTestExpr::And(left, right) | ast::ExtendedTestExpr::Or(left, right) => {
            check_extended_test(context, left)?;
            check_extended_test(context, right)
        }
        ast::ExtendedTestExpr::Not(expression)
        | ast::ExtendedTestExpr::Parenthesized(expression) => {
            check_extended_test(context, expression)
        }
        ast::ExtendedTestExpr::UnaryTest(_, word) => check_word(context, word),
        ast::ExtendedTestExpr::BinaryTest(_, left, right) => {
            check_word(context, left)?;
            check_word(context, right)
        }
    }
}

fn check_path(context: &Context, word: &ast::Word) -> Result<(), Error> {
    check_word(context, word)?;
    let result = (context.visitor.path)(word);
    policy_result(result)
}

pub fn executable_name(executable: &str) -> &str {
    match executable.rsplit_once('/') {
        Some((_, name)) => name,
        None => executable,
    }
}
