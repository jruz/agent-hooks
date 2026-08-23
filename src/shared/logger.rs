use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::PathBuf;

use serde::Serialize;
use thiserror::Error;

use crate::shared::env;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to create log dir: {0}")]
    CreateDir(std::io::Error),
    #[error("{0}")]
    Env(#[from] env::Error),
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
    let state_dir = env::state_home()?;
    let path = state_dir.join("agent-hooks");
    Ok(path)
}

fn log_path() -> Result<PathBuf, Error> {
    let dir = log_dir()?;
    let path = dir.join("hooks.log");
    Ok(path)
}

fn write_entry(entry: &LogEntry) -> Result<(), Error> {
    let dir = log_dir()?;
    fs::create_dir_all(&dir).map_err(Error::CreateDir)?;

    let line = serde_json::to_string(entry).map_err(Error::Serialize)?;

    let path = log_path()?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
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
