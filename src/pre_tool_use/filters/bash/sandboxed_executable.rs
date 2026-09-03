use std::io;

use brush_parser::{self, ast, word};

const SANDBOX_REQUIRED_EXECUTABLES: [&str; 27] = [
    "bash", "bun", "cargo", "chroot", "dash", "deno", "doas", "fish", "ksh", "nice", "node",
    "nohup", "npm", "npx", "nu", "nush", "parallel", "pnpm", "python", "python3", "setsid", "sudo",
    "sh", "time", "timeout", "xargs", "zsh",
];

#[derive(Debug, Clone, Copy)]
enum Wrapper {
    Command,
    Env,
    Eval,
    Exec,
    Find,
    NixShell,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Dynamic executable names must be invoked through `sandbox`")]
    DynamicExecutable,
    #[error(transparent)]
    Parse(#[from] brush_parser::ParseError),
    #[error(
        "Must be prefixed by `sandbox` (e.g. `sandbox cargo build`). Use `just` recipes when available."
    )]
    Unsandboxed,
    #[error(transparent)]
    WordParse(#[from] brush_parser::WordParseError),
}

pub fn run(command: &str) -> Result<(), Error> {
    let program = parse_program(command)?;
    check_program(&program)
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

fn parse_program(command: &str) -> Result<ast::Program, Error> {
    let bytes = command.as_bytes();
    let reader = io::Cursor::new(bytes);
    let options = parser_options();
    let mut parser = brush_parser::Parser::new(reader, &options);
    let program = parser.parse_program()?;
    Ok(program)
}

fn check_program(program: &ast::Program) -> Result<(), Error> {
    program
        .complete_commands
        .iter()
        .try_for_each(check_compound_list)
}

fn check_compound_list(list: &ast::CompoundList) -> Result<(), Error> {
    list.0
        .iter()
        .try_for_each(|item| check_and_or_list(&item.0))
}

fn check_and_or_list(list: &ast::AndOrList) -> Result<(), Error> {
    check_pipeline(&list.first)?;
    list.additional.iter().try_for_each(|item| match item {
        ast::AndOr::And(pipeline) | ast::AndOr::Or(pipeline) => check_pipeline(pipeline),
    })
}

fn check_pipeline(pipeline: &ast::Pipeline) -> Result<(), Error> {
    pipeline.seq.iter().try_for_each(check_command)
}

fn check_command(command: &ast::Command) -> Result<(), Error> {
    match command {
        ast::Command::Simple(command) => check_simple_command(command),
        ast::Command::Compound(command, redirects) => {
            check_compound_command(command)?;
            let redirects = redirects.as_ref();
            check_optional_redirects(redirects)
        }
        ast::Command::Function(function) => check_compound_command(&function.body.0),
        ast::Command::ExtendedTest(test, redirects) => {
            check_extended_test(&test.expr)?;
            let redirects = redirects.as_ref();
            check_optional_redirects(redirects)
        }
    }
}

fn check_simple_command(command: &ast::SimpleCommand) -> Result<(), Error> {
    let prefix = command.prefix.as_ref();
    let prefix = prefix_items(prefix);
    check_optional_items(prefix)?;

    let suffix = command.suffix.as_ref();
    let suffix_items = suffix_items(suffix);
    check_optional_items(suffix_items)?;
    let arguments = command_words(suffix);

    match command.word_or_name.as_ref() {
        Some(word) => check_executable_word(word, &arguments),
        None => Ok(()),
    }
}

fn command_words(suffix: Option<&ast::CommandSuffix>) -> Vec<&ast::Word> {
    suffix
        .into_iter()
        .flat_map(|suffix| &suffix.0)
        .filter_map(|item| match item {
            ast::CommandPrefixOrSuffixItem::Word(word) => Some(word),
            ast::CommandPrefixOrSuffixItem::IoRedirect(_)
            | ast::CommandPrefixOrSuffixItem::AssignmentWord(_, _)
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

fn check_executable_word(word: &ast::Word, arguments: &[&ast::Word]) -> Result<(), Error> {
    let executable = literal_word(word)?;
    let name = executable_name(&executable);

    if executable == "sandbox" {
        Ok(())
    } else if name == "sandbox" || requires_sandbox(&executable) {
        let error = Error::Unsandboxed;
        Err(error)
    } else {
        match wrapper(name) {
            Some(wrapper) => check_wrapper(wrapper, arguments),
            None => Ok(()),
        }
    }
}

fn wrapper(name: &str) -> Option<Wrapper> {
    match name {
        "command" => Some(Wrapper::Command),
        "env" => Some(Wrapper::Env),
        "eval" => Some(Wrapper::Eval),
        "exec" => Some(Wrapper::Exec),
        "find" => Some(Wrapper::Find),
        "nix-shell" => Some(Wrapper::NixShell),
        _ => None,
    }
}

fn check_wrapper(wrapper: Wrapper, arguments: &[&ast::Word]) -> Result<(), Error> {
    match wrapper {
        Wrapper::Command => check_command_wrapper(arguments),
        Wrapper::Env => check_env(arguments),
        Wrapper::Eval => check_eval(arguments),
        Wrapper::Exec => check_exec(arguments),
        Wrapper::Find => check_find(arguments),
        Wrapper::NixShell => check_nix_shell(arguments),
    }
}

fn check_eval(arguments: &[&ast::Word]) -> Result<(), Error> {
    let command = arguments
        .iter()
        .map(|word| literal_word(word))
        .collect::<Result<Vec<_>, _>>()?;
    let command = command.join(" ");
    let program = parse_program(&command)?;
    check_program(&program)
}

fn check_find(arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => check_find_argument(word, remaining),
        None => Ok(()),
    }
}

fn check_find_argument(word: &ast::Word, remaining: &[&ast::Word]) -> Result<(), Error> {
    let literal = static_word(word)?;
    if literal.as_deref().is_some_and(is_find_launcher) {
        match remaining.split_first() {
            Some((executable, executable_arguments)) => {
                check_executable_word(executable, executable_arguments)?;
                check_find(remaining)
            }
            None => Ok(()),
        }
    } else {
        check_find(remaining)
    }
}

fn is_find_launcher(argument: &str) -> bool {
    matches!(argument, "-exec" | "-execdir" | "-ok" | "-okdir")
}

fn check_nix_shell(arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => {
            let literal = static_word(word)?;
            match literal.as_deref() {
                Some("--run" | "--command") => check_source_argument(remaining),
                Some(argument) if argument.starts_with("--run=") => {
                    check_prefixed_source(argument, "--run=")
                }
                Some(argument) if argument.starts_with("--command=") => {
                    check_prefixed_source(argument, "--command=")
                }
                Some(_) | None => check_nix_shell(remaining),
            }
        }
        None => Ok(()),
    }
}

fn check_source_argument(arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, _)) => {
            let source = literal_word(word)?;
            check_source(&source)
        }
        None => Ok(()),
    }
}

fn check_prefixed_source(argument: &str, prefix: &str) -> Result<(), Error> {
    match argument.strip_prefix(prefix) {
        Some(source) => check_source(source),
        None => Ok(()),
    }
}

fn check_source(source: &str) -> Result<(), Error> {
    let program = parse_program(source)?;
    check_program(&program)
}

fn check_env(arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => check_env_argument(word, remaining),
        None => Ok(()),
    }
}

fn check_env_argument(word: &ast::Word, remaining: &[&ast::Word]) -> Result<(), Error> {
    let literal = static_word(word)?;
    match literal.as_deref() {
        Some("-S" | "--split-string") => check_source_argument(remaining),
        Some(argument) if argument.starts_with("--split-string=") => {
            check_prefixed_source(argument, "--split-string=")
        }
        Some(argument) if argument.starts_with("-S") => check_prefixed_source(argument, "-S"),
        Some("-a" | "--argv0" | "-u" | "--unset" | "-C" | "--chdir") => {
            check_env_after_option(remaining)
        }
        Some(argument) if argument.starts_with('-') || argument.contains('=') => {
            check_env(remaining)
        }
        Some(_) => check_executable_word(word, remaining),
        None if is_env_assignment(word) => check_env(remaining),
        None => Err(Error::DynamicExecutable),
    }
}

fn check_env_after_option(arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((_, remaining)) => check_env(remaining),
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

fn check_command_wrapper(arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => {
            let literal = literal_word(word)?;
            match literal.as_str() {
                "-p" | "--" => check_command_wrapper(remaining),
                "-v" | "-V" => Ok(()),
                _ => check_executable_word(word, remaining),
            }
        }
        None => Ok(()),
    }
}

fn check_exec(arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((word, remaining)) => {
            let literal = literal_word(word)?;
            match literal.as_str() {
                "-a" => check_exec_after_option(remaining),
                argument if argument.starts_with('-') => check_exec(remaining),
                _ => check_executable_word(word, remaining),
            }
        }
        None => Ok(()),
    }
}

fn check_exec_after_option(arguments: &[&ast::Word]) -> Result<(), Error> {
    match arguments.split_first() {
        Some((_, remaining)) => check_exec(remaining),
        None => Ok(()),
    }
}

fn literal_word(word: &ast::Word) -> Result<String, Error> {
    let Some(literal) = static_word(word)? else {
        return Err(Error::DynamicExecutable);
    };
    Ok(literal)
}

fn static_word(word: &ast::Word) -> Result<Option<String>, Error> {
    let pieces = parse_word(&word.value)?;
    check_word_pieces(&pieces)?;
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

fn parse_word(value: &str) -> Result<Vec<word::WordPieceWithSource>, Error> {
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
            .all(|nested_piece| is_static_piece(&nested_piece.piece)),
        word::WordPiece::AnsiCQuotedText(_)
        | word::WordPiece::GettextDoubleQuotedSequence(_)
        | word::WordPiece::TildeExpansion(_)
        | word::WordPiece::ParameterExpansion(_)
        | word::WordPiece::CommandSubstitution(_)
        | word::WordPiece::BackquotedCommandSubstitution(_)
        | word::WordPiece::ArithmeticExpression(_) => false,
    }
}

fn check_word(word: &ast::Word) -> Result<(), Error> {
    let pieces = parse_word(&word.value)?;
    check_word_pieces(&pieces)
}

fn check_word_pieces(pieces: &[word::WordPieceWithSource]) -> Result<(), Error> {
    pieces
        .iter()
        .try_for_each(|piece| check_word_piece(&piece.piece))
}

fn check_word_piece(piece: &word::WordPiece) -> Result<(), Error> {
    match piece {
        word::WordPiece::DoubleQuotedSequence(pieces)
        | word::WordPiece::GettextDoubleQuotedSequence(pieces) => check_word_pieces(pieces),
        word::WordPiece::CommandSubstitution(command)
        | word::WordPiece::BackquotedCommandSubstitution(command) => {
            let program = parse_program(command)?;
            check_program(&program)
        }
        word::WordPiece::ParameterExpansion(expression) => check_parameter_expression(expression),
        word::WordPiece::ArithmeticExpression(expression) => check_embedded_word(&expression.value),
        word::WordPiece::Text(_)
        | word::WordPiece::SingleQuotedText(_)
        | word::WordPiece::AnsiCQuotedText(_)
        | word::WordPiece::TildeExpansion(_)
        | word::WordPiece::EscapeSequence(_) => Ok(()),
    }
}

fn check_parameter_expression(expression: &word::ParameterExpr) -> Result<(), Error> {
    check_parameter(expression)?;
    match expression {
        word::ParameterExpr::UseDefaultValues { default_value, .. }
        | word::ParameterExpr::AssignDefaultValues { default_value, .. } => {
            let default_value = default_value.as_deref();
            check_optional_embedded_word(default_value)
        }
        word::ParameterExpr::IndicateErrorIfNullOrUnset { error_message, .. } => {
            let error_message = error_message.as_deref();
            check_optional_embedded_word(error_message)
        }
        word::ParameterExpr::UseAlternativeValue {
            alternative_value, ..
        } => {
            let alternative_value = alternative_value.as_deref();
            check_optional_embedded_word(alternative_value)
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
            check_optional_embedded_word(pattern)
        }
        word::ParameterExpr::Substring { offset, length, .. } => {
            check_embedded_word(&offset.value)?;
            match length {
                Some(length) => check_embedded_word(&length.value),
                None => Ok(()),
            }
        }
        word::ParameterExpr::ReplaceSubstring {
            pattern,
            replacement,
            ..
        } => {
            check_embedded_word(pattern)?;
            let replacement = replacement.as_deref();
            check_optional_embedded_word(replacement)
        }
        word::ParameterExpr::Parameter { .. }
        | word::ParameterExpr::ParameterLength { .. }
        | word::ParameterExpr::Transform { .. }
        | word::ParameterExpr::VariableNames { .. }
        | word::ParameterExpr::MemberKeys { .. } => Ok(()),
    }
}

fn check_parameter(expression: &word::ParameterExpr) -> Result<(), Error> {
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
        Some(word::Parameter::NamedWithIndex { index, .. }) => check_embedded_word(index),
        Some(
            word::Parameter::Positional(_)
            | word::Parameter::Special(_)
            | word::Parameter::Named(_)
            | word::Parameter::NamedWithAllIndices { .. },
        )
        | None => Ok(()),
    }
}

fn check_optional_embedded_word(value: Option<&str>) -> Result<(), Error> {
    match value {
        Some(value) => check_embedded_word(value),
        None => Ok(()),
    }
}

fn check_embedded_word(value: &str) -> Result<(), Error> {
    let pieces = parse_word(value)?;
    check_word_pieces(&pieces)
}

fn check_optional_items(items: Option<&[ast::CommandPrefixOrSuffixItem]>) -> Result<(), Error> {
    match items {
        Some(items) => items.iter().try_for_each(check_item),
        None => Ok(()),
    }
}

fn check_item(item: &ast::CommandPrefixOrSuffixItem) -> Result<(), Error> {
    match item {
        ast::CommandPrefixOrSuffixItem::IoRedirect(redirect) => check_redirect(redirect),
        ast::CommandPrefixOrSuffixItem::Word(word) => check_word(word),
        ast::CommandPrefixOrSuffixItem::AssignmentWord(assignment, word) => {
            check_assignment(assignment)?;
            check_word(word)
        }
        ast::CommandPrefixOrSuffixItem::ProcessSubstitution(_, command) => {
            check_compound_list(&command.list)
        }
    }
}

fn check_assignment(assignment: &ast::Assignment) -> Result<(), Error> {
    match &assignment.value {
        ast::AssignmentValue::Scalar(word) => check_word(word),
        ast::AssignmentValue::Array(values) => values.iter().try_for_each(|(key, value)| {
            if let Some(key) = key {
                check_word(key)?;
            }
            check_word(value)
        }),
    }
}

fn check_optional_redirects(redirects: Option<&ast::RedirectList>) -> Result<(), Error> {
    match redirects {
        Some(redirects) => redirects.0.iter().try_for_each(check_redirect),
        None => Ok(()),
    }
}

fn check_redirect(redirect: &ast::IoRedirect) -> Result<(), Error> {
    match redirect {
        ast::IoRedirect::File(_, _, target) => check_redirect_target(target),
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
            check_word_pieces(&pieces)
        }
        ast::IoRedirect::HereDocument(_, _) => Ok(()),
        ast::IoRedirect::HereString(_, word) | ast::IoRedirect::OutputAndError(word, _) => {
            check_word(word)
        }
    }
}

fn check_redirect_target(target: &ast::IoFileRedirectTarget) -> Result<(), Error> {
    match target {
        ast::IoFileRedirectTarget::Filename(word) | ast::IoFileRedirectTarget::Duplicate(word) => {
            check_word(word)
        }
        ast::IoFileRedirectTarget::Fd(_) => Ok(()),
        ast::IoFileRedirectTarget::ProcessSubstitution(_, command) => {
            check_compound_list(&command.list)
        }
    }
}

fn check_compound_command(command: &ast::CompoundCommand) -> Result<(), Error> {
    match command {
        ast::CompoundCommand::Arithmetic(command) => check_embedded_word(&command.expr.value),
        ast::CompoundCommand::ArithmeticForClause(command) => {
            let initializer = command.initializer.as_ref();
            check_optional_arithmetic(initializer)?;
            let condition = command.condition.as_ref();
            check_optional_arithmetic(condition)?;
            let updater = command.updater.as_ref();
            check_optional_arithmetic(updater)?;
            check_compound_list(&command.body.list)
        }
        ast::CompoundCommand::BraceGroup(command) => check_compound_list(&command.list),
        ast::CompoundCommand::Subshell(command) => check_compound_list(&command.list),
        ast::CompoundCommand::ForClause(command) => {
            if let Some(values) = command.values.as_ref() {
                values.iter().try_for_each(check_word)?;
            }
            check_compound_list(&command.body.list)
        }
        ast::CompoundCommand::CaseClause(command) => {
            check_word(&command.value)?;
            command.cases.iter().try_for_each(|case| {
                case.patterns.iter().try_for_each(check_word)?;
                match case.cmd.as_ref() {
                    Some(case_command) => check_compound_list(case_command),
                    None => Ok(()),
                }
            })
        }
        ast::CompoundCommand::IfClause(command) => {
            check_compound_list(&command.condition)?;
            check_compound_list(&command.then)?;
            if let Some(elses) = command.elses.as_ref() {
                elses.iter().try_for_each(|clause| {
                    if let Some(condition) = clause.condition.as_ref() {
                        check_compound_list(condition)?;
                    }
                    check_compound_list(&clause.body)
                })?;
            }
            Ok(())
        }
        ast::CompoundCommand::WhileClause(command) | ast::CompoundCommand::UntilClause(command) => {
            check_compound_list(&command.0)?;
            check_compound_list(&command.1.list)
        }
        ast::CompoundCommand::Coprocess(command) => check_command(&command.body),
    }
}

fn check_optional_arithmetic(
    expression: Option<&ast::UnexpandedArithmeticExpr>,
) -> Result<(), Error> {
    match expression {
        Some(expression) => check_embedded_word(&expression.value),
        None => Ok(()),
    }
}

fn check_extended_test(expression: &ast::ExtendedTestExpr) -> Result<(), Error> {
    match expression {
        ast::ExtendedTestExpr::And(left, right) | ast::ExtendedTestExpr::Or(left, right) => {
            check_extended_test(left)?;
            check_extended_test(right)
        }
        ast::ExtendedTestExpr::Not(expression)
        | ast::ExtendedTestExpr::Parenthesized(expression) => check_extended_test(expression),
        ast::ExtendedTestExpr::UnaryTest(_, word) => check_word(word),
        ast::ExtendedTestExpr::BinaryTest(_, left, right) => {
            check_word(left)?;
            check_word(right)
        }
    }
}

fn requires_sandbox(executable: &str) -> bool {
    let name = executable_name(executable);
    SANDBOX_REQUIRED_EXECUTABLES.contains(&name)
}

fn executable_name(executable: &str) -> &str {
    match executable.rsplit_once('/') {
        Some((_, name)) => name,
        None => executable,
    }
}
