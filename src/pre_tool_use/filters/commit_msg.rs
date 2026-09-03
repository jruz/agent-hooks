use regex::Regex;

const COMMIT_TITLE_PATTERN: &str = r"^\[(feat|bugs|refa|deps|tool)\] \S+.*: .+";

pub fn run(command: &str) -> Result<(), String> {
    if is_git_commit(command) && !should_skip(command) {
        let title = extract_title(command);

        let Some(title) = title else {
            let reason = "Could not extract commit title".to_string();
            return Err(reason);
        };

        if !validate_title(&title) {
            let reason = "Invalid commit title, follow the Skill(commit) format".to_string();
            return Err(reason);
        }
    }

    Ok(())
}

fn is_git_commit(command: &str) -> bool {
    command.contains("git commit")
}

fn should_skip(command: &str) -> bool {
    command.contains("--no-edit") || command.contains("--fixup")
}

fn extract_inline_title(command: &str) -> Option<String> {
    let patterns = [
        r#"git commit\s+.*?-m\s+"([^"]+)""#,
        r"git commit\s+.*?-m\s+'([^']+)'",
    ];

    patterns.iter().find_map(|pattern| {
        let re = Regex::new(pattern).ok()?;
        let caps = re.captures(command)?;
        let first_line = caps.get(1)?.as_str().lines().next()?.trim();
        (!first_line.is_empty()).then(|| first_line.to_string())
    })
}

fn extract_title(command: &str) -> Option<String> {
    let is_heredoc = command.contains("$(cat <<");

    if !is_heredoc && let Some(title) = extract_inline_title(command) {
        return Some(title);
    }

    command
        .lines()
        .find(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && !line.starts_with("git commit")
                && !trimmed.starts_with("$(cat")
                && !trimmed.starts_with("Co-Authored")
                && !trimmed.starts_with("EOF")
                && !trimmed.starts_with(")\"")
        })
        .map(|line| line.trim().to_string())
}

fn validate_title(title: &str) -> bool {
    Regex::new(COMMIT_TITLE_PATTERN).is_ok_and(|re| re.is_match(title))
}
