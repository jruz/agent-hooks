mod curl;
mod environment;
mod paths;
mod rules;
mod sandboxed_executable;

use brush_parser::ast;

use super::{commit_msg, shell};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Commit(#[from] commit_msg::Error),
    #[error(transparent)]
    Curl(#[from] curl::Error),
    #[error(transparent)]
    Environment(#[from] environment::Error),
    #[error(transparent)]
    Paths(#[from] paths::Error),
    #[error(transparent)]
    Rules(#[from] rules::Error),
    #[error(transparent)]
    Sandbox(#[from] sandboxed_executable::Error),
    #[error(transparent)]
    Shell(#[from] shell::Error),
}

pub fn run(command: &str) -> Result<(), String> {
    let visitor = shell::Visitor {
        assignment: check_assignment,
        invocation: check_invocation,
        output: check_output,
        path: check_redirect,
    };
    match shell::run(command, &visitor) {
        Ok(()) => Ok(()),
        Err(error) => {
            let reason = error.to_string();
            Err(reason)
        }
    }
}

fn policy_reason<Failure>(result: Result<(), Failure>) -> Result<(), String>
where
    Failure: std::fmt::Display,
{
    match result {
        Ok(()) => Ok(()),
        Err(error) => {
            let reason = error.to_string();
            Err(reason)
        }
    }
}

fn check_assignment(assignment: &ast::Assignment) -> Result<(), String> {
    let result = rules::check_assignment(assignment);
    policy_reason(result)
}

fn check_output(output: shell::Output<'_>) -> Result<(), String> {
    let result = environment::check_output(output);
    policy_reason(result)
}

fn check_redirect(word: &ast::Word) -> Result<(), String> {
    let result = paths::check_word(word);
    policy_reason(result)
}

fn check_invocation(
    executable: &str,
    arguments: &[&ast::Word],
    sandbox: shell::Sandbox,
    execution: shell::Execution,
    stdin: Option<&str>,
) -> Result<(), String> {
    let result = check_command(executable, arguments, sandbox, execution, stdin);
    policy_reason(result)
}

fn check_command(
    executable: &str,
    arguments: &[&ast::Word],
    sandbox: shell::Sandbox,
    execution: shell::Execution,
    stdin: Option<&str>,
) -> Result<(), Error> {
    let name = shell::executable_name(executable);
    let arguments = if name == "git" {
        git_arguments(arguments)?
    } else {
        arguments
    };
    let literals = arguments
        .iter()
        .map(|word| shell::static_word(word))
        .collect::<Result<Vec<_>, _>>()?;
    rules::run(name, &literals, execution)?;
    rules::check_bindings(name, arguments)?;
    environment::run(name, arguments)?;
    paths::run(name, arguments)?;
    if name == "curl" {
        curl::run(arguments)?;
    }
    if name == "git" && literals.first().and_then(Option::as_deref) == Some("commit") {
        commit_msg::run(arguments, stdin)?;
    }
    if execution == shell::Execution::Local {
        sandboxed_executable::run(executable, sandbox)?;
    }
    Ok(())
}

fn git_arguments<'words>(
    arguments: &'words [&ast::Word],
) -> Result<&'words [&'words ast::Word], Error> {
    match arguments.split_first() {
        Some((word, remaining)) => {
            let literal = shell::static_word(word)?;
            match literal.as_deref() {
                Some(
                    "-C" | "-c" | "--git-dir" | "--work-tree" | "--namespace" | "--config-env",
                ) => match remaining.split_first() {
                    Some((_, remaining)) => git_arguments(remaining),
                    None => Ok(remaining),
                },
                Some(argument) if argument.starts_with('-') => git_arguments(remaining),
                Some(_) | None => Ok(arguments),
            }
        }
        None => Ok(arguments),
    }
}
