use std::process::Command;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum JustFixError {
    #[error("failed to run `just fix`: {0}")]
    CommandFailed(#[from] std::io::Error),
}

pub fn run() -> Result<(), JustFixError> {
    Command::new("just").arg("fix").status()?;
    Ok(())
}
