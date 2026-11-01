use brush_parser::ast;

use super::super::shell;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Blocked(&'static str),
}

pub fn run(
    executable: &str,
    arguments: &[Option<String>],
    execution: shell::Execution,
) -> Result<(), Error> {
    let first = argument(arguments, 0);
    let second = argument(arguments, 1);
    let reason = match executable {
        "cargo" if first == Some("install") && has_argument(arguments, "--path") => {
            Some("Use `just install` instead")
        }
        "npm" if first == Some("run") => Some("Use `just <task>` instead of `npm run`"),
        "npx" if first == Some("playwright") && second == Some("install") => {
            Some("Use `just playwright-install` instead")
        }
        "npx" if first == Some("run") => Some("Use `just <task>` instead of `npx run`"),
        "age" if has_argument(arguments, "--decrypt") => {
            Some("Access to sensitive command blocked")
        }
        "sops"
            if ["decrypt", "exec-env", "exec-file", "--decrypt", "-d"]
                .iter()
                .any(|argument| has_argument(arguments, argument)) =>
        {
            Some(
                "Direct SOPS decryption is blocked. Read the encrypted YAML directly to inspect keys and unencrypted values. Use `just sops-decrypt-dev` to generate `.env` and `just dotenv-show` for masked inspection, where these recipes exist.",
            )
        }
        "gleam" if first == Some("run") => Some("Use just run instead"),
        "security" => Some("Access to macOS security command blocked"),
        "history" | "fc" => Some("Access to shell history blocked"),
        "atuin" if matches!(first, Some("search" | "history" | "stats" | "key")) => {
            Some("Access to shell history blocked")
        }
        "git" if first == Some("filter-branch") && has_argument(arguments, "--all") => Some(
            "Never use --all with git filter-branch \u{2014} it overwrites backup branches. Target specific branches instead.",
        ),
        "env" if execution == shell::Execution::Container => {
            Some("Never read container environment variables")
        }
        "env"
            if arguments
                .iter()
                .flatten()
                .any(|argument| argument.starts_with("SKIP_HOOKS=")) =>
        {
            Some("Don't skip hooks")
        }
        _ => snapshot_reason(executable, arguments),
    };
    match reason {
        Some(reason) => {
            let error = Error::Blocked(reason);
            Err(error)
        }
        None => Ok(()),
    }
}

fn snapshot_reason(executable: &str, arguments: &[Option<String>]) -> Option<&'static str> {
    let test_launcher = matches!(
        executable,
        "bun"
            | "cargo"
            | "just"
            | "npm"
            | "npx"
            | "playwright"
            | "pnpm"
            | "pytest"
            | "python"
            | "python3"
            | "uv"
    );
    let updates_snapshots = test_launcher
        && (has_argument(arguments, "--update-snapshots")
            || has_argument(arguments, "--update_snapshots"));
    updates_snapshots.then_some("Only run update-snapshots after investigating the test results and confirming the diff is not a bug. Ask the user to run this command manually.")
}

pub fn check_bindings(executable: &str, arguments: &[&ast::Word]) -> Result<(), Error> {
    let binds_variables = matches!(
        executable,
        "declare" | "env" | "export" | "local" | "readonly" | "typeset"
    );
    let skips_hooks = binds_variables
        && arguments.iter().any(|word| {
            let value = brush_parser::unquote_str(&word.value);
            value == "SKIP_HOOKS" || value.starts_with("SKIP_HOOKS=")
        });
    if skips_hooks {
        let error = Error::Blocked("Don't skip hooks");
        Err(error)
    } else {
        Ok(())
    }
}

pub fn check_assignment(assignment: &ast::Assignment) -> Result<(), Error> {
    let name = match &assignment.name {
        ast::AssignmentName::VariableName(name)
        | ast::AssignmentName::ArrayElementName(name, _) => name,
    };
    if name == "SKIP_HOOKS" {
        let error = Error::Blocked("Don't skip hooks");
        Err(error)
    } else {
        Ok(())
    }
}

fn argument(arguments: &[Option<String>], index: usize) -> Option<&str> {
    arguments.get(index).and_then(Option::as_deref)
}

fn has_argument(arguments: &[Option<String>], expected: &str) -> bool {
    arguments
        .iter()
        .flatten()
        .any(|argument| argument == expected)
}
