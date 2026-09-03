mod sandboxed_executable;

use regex::Regex;

struct Rule {
    matches: fn(&str) -> bool,
    message: &'static str,
}

pub fn run(command: &str) -> Result<(), String> {
    let matching_rule = rules().into_iter().find(|rule| (rule.matches)(command));

    match matching_rule {
        Some(rule) => {
            let message = rule.message.to_string();
            Err(message)
        }
        None => sandboxed_executable::run(command).map_err(|error| error.to_string()),
    }
}

fn is_envrc(command: &str) -> bool {
    command.contains(".envrc")
}

fn has_bare_dot_env(command: &str) -> bool {
    if is_envrc(command) {
        return false;
    }
    Regex::new(r"^git\s+(add|commit)\b").is_ok_and(|exclude| {
        if exclude.is_match(command) {
            return false;
        }
        command.match_indices(".env").any(|(idx, _)| {
            let after_idx = idx + ".env".len();
            command.get(after_idx..after_idx + 1) != Some(".")
        })
    })
}

fn rules() -> Vec<Rule> {
    vec![
        Rule {
            matches: |cmd| cmd.contains("cargo install --path"),
            message: "Use `just install` instead",
        },
        Rule {
            matches: |cmd| cmd.contains("git -C"),
            message: "Use relative paths instead of `git -C`",
        },
        Rule {
            matches: |cmd| cmd.contains("npm run"),
            message: "Use `just <task>` instead of `npm run`",
        },
        Rule {
            matches: |cmd| cmd.contains("npx playwright install"),
            message: "Use `just playwright-install` instead",
        },
        Rule {
            matches: |cmd| cmd.contains("npx run"),
            message: "Use `just <task>` instead of `npx run`",
        },
        Rule {
            matches: |cmd| cmd.contains("SKIP_HOOKS"),
            message: "Don't skip hooks",
        },
        Rule {
            matches: |cmd| Regex::new(r"docker exec.*\benv\b").is_ok_and(|re| re.is_match(cmd)),
            message: "Never read container environment variables",
        },
        Rule {
            matches: |cmd| cmd.contains("age --decrypt"),
            message: "Access to sensitive command blocked",
        },
        Rule {
            matches: |cmd| {
                Regex::new(r"sops (-d\b|--decrypt|decrypt)").is_ok_and(|re| re.is_match(cmd))
            },
            message: "Don't decrypt sops files, just read them directly",
        },
        Rule {
            matches: has_bare_dot_env,
            message: "Never access .env files directly",
        },
        Rule {
            matches: |cmd| {
                [
                    "/etc/shadow",
                    "/etc/passwd",
                    "/etc/gshadow",
                    "/etc/sudoers",
                    "/etc/master.passwd",
                ]
                .iter()
                .any(|path| cmd.contains(path))
            },
            message: "Access to system secret file blocked",
        },
        Rule {
            matches: |cmd| cmd.starts_with("gleam run"),
            message: "Use just run instead",
        },
        Rule {
            matches: |cmd| Regex::new(r"\bsecurity\b").is_ok_and(|re| re.is_match(cmd)),
            message: "Access to macOS security command blocked",
        },
        Rule {
            matches: |cmd| cmd.contains("update-snapshots") || cmd.contains("update_snapshots"),
            message: "Only run update-snapshots after investigating the test results and confirming the diff is not a bug. Ask the user to run this command manually.",
        },
        Rule {
            matches: |cmd| Regex::new("^git filter-branch.*--all").is_ok_and(|re| re.is_match(cmd)),
            message: "Never use --all with git filter-branch \u{2014} it overwrites backup branches. Target specific branches instead.",
        },
        Rule {
            matches: |cmd| {
                Regex::new(r"^git\s+commit\b").is_ok_and(|re| re.is_match(cmd))
                    && cmd.contains("Co-Authored-By")
            },
            message: "Never include Co-Authored-By in commit messages",
        },
    ]
}
