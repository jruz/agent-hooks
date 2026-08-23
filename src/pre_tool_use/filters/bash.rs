use regex::Regex;

const SANDBOXED_EXECUTABLES: [&str; 9] = [
    "cargo", "bun", "pnpm", "node", "python", "python3", "npm", "npx", "deno",
];

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
        None => Ok(()),
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

fn is_executable_name_character(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '_' | '-')
}

fn is_executable_path_character(character: char) -> bool {
    is_executable_name_character(character) || character == '/'
}

fn is_executable_name(command: &str, index: usize, executable: &str) -> bool {
    let before = command
        .get(..index)
        .and_then(|prefix| prefix.chars().next_back());
    let after_index = index + executable.len();
    let after = command
        .get(after_index..)
        .and_then(|suffix| suffix.chars().next());

    !before.is_some_and(is_executable_name_character)
        && !after.is_some_and(is_executable_path_character)
}

fn is_path_character(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-')
}

fn sandbox_precedes(command: &str, executable_index: usize) -> bool {
    let bytes = command.as_bytes();
    let mut cursor = executable_index;

    if cursor > 0 && bytes.get(cursor - 1) == Some(&b'/') {
        while cursor > 0
            && bytes
                .get(cursor - 1)
                .is_some_and(|byte| is_path_character(*byte))
        {
            cursor -= 1;
        }
    }

    if cursor > 0 && matches!(bytes.get(cursor - 1), Some(b'\'' | b'"')) {
        cursor -= 1;
    }

    while cursor > 0 && bytes.get(cursor - 1).is_some_and(u8::is_ascii_whitespace) {
        cursor -= 1;
    }

    let sandbox_end = cursor;
    while cursor > 0 && bytes.get(cursor - 1).is_some_and(u8::is_ascii_alphabetic) {
        cursor -= 1;
    }

    command.get(cursor..sandbox_end) == Some("sandbox")
}

fn has_unsandboxed_executable(command: &str) -> bool {
    SANDBOXED_EXECUTABLES.iter().any(|executable| {
        command.match_indices(executable).any(|(index, _)| {
            is_executable_name(command, index, executable) && !sandbox_precedes(command, index)
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
        Rule {
            matches: has_unsandboxed_executable,
            message: "Must be prefixed by `sandbox` (e.g. `sandbox cargo build`). Use `just` recipes when available.",
        },
    ]
}
