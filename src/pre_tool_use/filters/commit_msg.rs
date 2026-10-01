use regex::Regex;

const COMMIT_TITLE_PATTERN: &str = r"^\[(feat|bugs|refa|deps|tool)\] \S+.*: .+";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid commit title, follow the Skill(commit) format")]
    InvalidTitle,
    #[error("Could not extract commit title")]
    MissingTitle,
    #[error(transparent)]
    Regex(#[from] regex::Error),
}

pub fn run(command: &str) -> Result<(), String> {
    match check_commit(command) {
        Ok(()) => Ok(()),
        Err(error) => {
            let reason = error.to_string();
            Err(reason)
        }
    }
}

fn check_commit(command: &str) -> Result<(), Error> {
    if is_git_commit(command) && !should_skip(command) {
        let title = extract_title(command)?;

        let Some(title) = title else {
            return Err(Error::MissingTitle);
        };

        if !validate_title(&title)? {
            return Err(Error::InvalidTitle);
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

fn extract_inline_title(command: &str) -> Result<Option<String>, Error> {
    let patterns = [
        r#"git commit\s+.*?-m\s+"([^"]+)""#,
        r"git commit\s+.*?-m\s+'([^']+)'",
    ];

    let expressions = patterns
        .iter()
        .map(|pattern| Regex::new(pattern))
        .collect::<Result<Vec<_>, _>>()?;
    let title = expressions.iter().find_map(|re| {
        let caps = re.captures(command)?;
        let first_line = caps.get(1)?.as_str().lines().next()?.trim();
        (!first_line.is_empty()).then(|| first_line.to_string())
    });
    Ok(title)
}

fn extract_title(command: &str) -> Result<Option<String>, Error> {
    let is_heredoc = command.contains("$(cat <<");

    if !is_heredoc && let Some(title) = extract_inline_title(command)? {
        let title = Some(title);
        return Ok(title);
    }

    let title = command
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
        .map(|line| line.trim().to_string());
    Ok(title)
}

fn validate_title(title: &str) -> Result<bool, Error> {
    let expression = Regex::new(COMMIT_TITLE_PATTERN)?;
    let valid = expression.is_match(title);
    Ok(valid)
}
