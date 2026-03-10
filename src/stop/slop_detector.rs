use std::process::Command;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SlopDetectorError {
    #[error("failed to run slop-detector: {0}")]
    CommandFailed(#[from] std::io::Error),
}

pub fn run() -> Result<Option<String>, SlopDetectorError> {
    let output = Command::new("slop-detector").output()?;

    if output.status.success() {
        return Ok(None);
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stderr}\n{stdout}").trim().to_string();
    Ok(Some(combined))
}
