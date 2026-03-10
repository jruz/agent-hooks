use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to create signal dir: {0}")]
    CreateDir(std::io::Error),
    #[error("failed to get uid: {0}")]
    Uid(std::io::Error),
    #[error("failed to write signal file: {0}")]
    WriteSignal(std::io::Error),
}

fn temp_dir() -> PathBuf {
    match env::var("XDG_RUNTIME_DIR").or_else(|_| env::var("TMPDIR")) {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => PathBuf::from("/tmp"),
    }
}

fn uid() -> Result<String, Error> {
    let output = Command::new("id").arg("-u").output().map_err(Error::Uid)?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn write_signal(content: &str) -> Result<(), Error> {
    let (Ok(pane_id), Ok(session)) = (env::var("ZELLIJ_PANE_ID"), env::var("ZELLIJ_SESSION_NAME"))
    else {
        return Ok(());
    };

    let dir = temp_dir()
        .join(format!("zellij-{}", uid()?))
        .join("tab-renamer-signals")
        .join(session);

    fs::create_dir_all(&dir).map_err(Error::CreateDir)?;
    fs::write(dir.join(pane_id), content).map_err(Error::WriteSignal)?;
    Ok(())
}
