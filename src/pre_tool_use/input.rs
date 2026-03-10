use std::io::{self, Read as _};

use serde::Deserialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InputError {
    #[error("failed to read stdin: {0}")]
    Io(#[from] io::Error),
    #[error("failed to parse stdin JSON: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Deserialize)]
#[serde(tag = "tool_name", content = "tool_input")]
pub enum ToolInput {
    Bash { command: String },
    Grep { file_path: String },
    Read { file_path: String },
}

pub fn read_stdin() -> Result<ToolInput, InputError> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    let input: ToolInput = serde_json::from_str(&buf)?;
    Ok(input)
}
