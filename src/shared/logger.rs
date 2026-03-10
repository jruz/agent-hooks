use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::PathBuf;

use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to create log dir: {0}")]
    CreateDir(std::io::Error),
    #[error("HOME not set: {0}")]
    HomeNotSet(std::env::VarError),
    #[error("failed to open log file: {0}")]
    OpenFile(std::io::Error),
    #[error("failed to serialize log entry: {0}")]
    Serialize(serde_json::Error),
    #[error("failed to write log entry: {0}")]
    Write(std::io::Error),
}

#[derive(Serialize)]
struct LogEntry {
    #[serde(rename = "Attributes")]
    attributes: HashMap<String, String>,
    #[serde(rename = "Body")]
    body: String,
    #[serde(rename = "SeverityText")]
    severity_text: &'static str,
    #[serde(rename = "Timestamp")]
    timestamp: String,
}

fn log_dir() -> Result<PathBuf, Error> {
    let state_dir = std::env::var("XDG_STATE_HOME")
        .or_else(|_| std::env::var("HOME").map(|home| format!("{home}/.local/state")))
        .map_err(Error::HomeNotSet)?;
    Ok(PathBuf::from(state_dir).join("claude-hooks"))
}

fn log_path() -> Result<PathBuf, Error> {
    Ok(log_dir()?.join("hooks.log"))
}

fn write_entry(entry: &LogEntry) -> Result<(), Error> {
    let dir = log_dir()?;
    fs::create_dir_all(&dir).map_err(Error::CreateDir)?;

    let line = serde_json::to_string(entry).map_err(Error::Serialize)?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path()?)
        .map_err(Error::OpenFile)?;

    writeln!(file, "{line}").map_err(Error::Write)?;
    Ok(())
}

pub fn error(body: &str, attributes: HashMap<String, String>) -> Result<(), Error> {
    let entry = LogEntry {
        attributes,
        body: body.to_owned(),
        severity_text: "ERROR",
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    write_entry(&entry)
}
