use std::env;
use std::fs;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClaudeTreesError {
    #[error("failed to write signal file: {0}")]
    WriteSignal(#[from] std::io::Error),
}

fn write_signal(signal_dir: &str, tab_id: &str) -> Result<(), ClaudeTreesError> {
    let signal_path = PathBuf::from(signal_dir).join(format!("tab_{tab_id}"));
    fs::write(signal_path, "")?;
    Ok(())
}

pub fn run() -> Result<(), ClaudeTreesError> {
    if let (Ok(signal_dir), Ok(tab_id)) = (
        env::var("CLAUDE_TREES_SIGNAL_DIR"),
        env::var("CLAUDE_TREES_TAB_ID"),
    ) {
        write_signal(&signal_dir, &tab_id)?;
    }
    Ok(())
}
