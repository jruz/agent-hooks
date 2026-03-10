use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ZellijError {
    #[error("failed to create signal dir: {0}")]
    CreateDir(std::io::Error),
    #[error("failed to get temp dir: {0}")]
    TempDir(std::io::Error),
    #[error("failed to get uid: {0}")]
    Uid(std::io::Error),
    #[error("failed to write signal file: {0}")]
    WriteSignal(std::io::Error),
}

fn get_darwin_temp_dir() -> Result<String, ZellijError> {
    let output = Command::new("getconf")
        .arg("DARWIN_USER_TEMP_DIR")
        .output()
        .map_err(ZellijError::TempDir)?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn get_uid() -> Result<String, ZellijError> {
    let output = Command::new("id")
        .arg("-u")
        .output()
        .map_err(ZellijError::Uid)?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn write_signal(pane_id: &str, session: &str) -> Result<(), ZellijError> {
    let tmpdir = get_darwin_temp_dir()?;
    let uid = get_uid()?;
    let dir = PathBuf::from(tmpdir)
        .join(format!("zellij-{uid}"))
        .join("tab-renamer-signals")
        .join(session);

    fs::create_dir_all(&dir).map_err(ZellijError::CreateDir)?;
    fs::write(dir.join(pane_id), "question").map_err(ZellijError::WriteSignal)?;
    Ok(())
}

pub fn run() -> Result<(), ZellijError> {
    if let (Ok(pane_id), Ok(session)) =
        (env::var("ZELLIJ_PANE_ID"), env::var("ZELLIJ_SESSION_NAME"))
    {
        write_signal(&pane_id, &session)?;
    }
    Ok(())
}
