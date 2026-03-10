mod claude_trees;
mod just_check;
mod just_fix;
mod slop_detector;
mod zellij;

use std::path::Path;

use thiserror::Error;

use crate::shared::response;

#[derive(Error, Debug)]
pub enum StopError {
    #[error("{0}")]
    ClaudeTrees(#[from] claude_trees::ClaudeTreesError),
    #[error("{0}")]
    JustCheck(#[from] just_check::JustCheckError),
    #[error("{0}")]
    JustFix(#[from] just_fix::JustFixError),
    #[error("{0}")]
    Serialize(#[from] serde_json::Error),
    #[error("{0}")]
    SlopDetector(#[from] slop_detector::SlopDetectorError),
    #[error("{0}")]
    Zellij(#[from] crate::shared::zellij::Error),
}

fn justfile_exists() -> bool {
    Path::new("justfile").exists()
}

pub fn run() -> Result<Option<String>, StopError> {
    claude_trees::run()?;
    zellij::run()?;

    if let Some(output) = slop_detector::run()? {
        let json = response::block(format!("slop-detector failed:\n{output}"))?;
        return Ok(Some(json));
    }

    if justfile_exists() {
        just_fix::run()?;

        if let Some(output) = just_check::run()? {
            let json = response::block(format!("just check failed:\n{output}"))?;
            return Ok(Some(json));
        }
    }

    Ok(None)
}
