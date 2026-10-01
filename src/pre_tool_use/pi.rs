use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

use crate::shared::response;

use super::filters;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to parse Pi PreToolUse JSON: {0}")]
    Json(serde_json::Error),
    #[error("failed to serialize Pi PreToolUse response: {0}")]
    Serialize(serde_json::Error),
}

#[derive(Deserialize)]
struct ToolCall {
    input: Value,
    #[serde(rename = "toolName")]
    tool_name: String,
}

#[derive(Deserialize)]
struct BashInput {
    command: String,
}

#[derive(Deserialize)]
struct ReadInput {
    path: String,
}

#[derive(Deserialize)]
struct GrepInput {
    path: Option<String>,
}

pub fn run(stdin: &str) -> Result<Option<String>, Error> {
    let tool_call = parse_input(stdin)?;
    match tool_call.tool_name.as_str() {
        "bash" => run_bash(tool_call.input),
        "read" => run_read(tool_call.input),
        "grep" => run_grep(tool_call.input),
        _ => Ok(None),
    }
}

fn parse_input(stdin: &str) -> Result<ToolCall, Error> {
    serde_json::from_str(stdin).map_err(Error::Json)
}

fn run_bash(input: Value) -> Result<Option<String>, Error> {
    let bash: BashInput = serde_json::from_value(input).map_err(Error::Json)?;
    let result = filters::bash::run(&bash.command);
    build_response(result)
}

fn run_read(input: Value) -> Result<Option<String>, Error> {
    let read: ReadInput = serde_json::from_value(input).map_err(Error::Json)?;
    let result = filters::secrets::run(&read.path);
    build_response(result)
}

fn run_grep(input: Value) -> Result<Option<String>, Error> {
    let grep: GrepInput = serde_json::from_value(input).map_err(Error::Json)?;
    match grep.path {
        Some(path) => {
            let result = filters::secrets::run(&path);
            build_response(result)
        }
        None => Ok(None),
    }
}

fn build_response(result: Result<(), String>) -> Result<Option<String>, Error> {
    match result {
        Ok(()) => Ok(None),
        Err(reason) => response::pre_tool_use_pi_block(reason)
            .map(Some)
            .map_err(Error::Serialize),
    }
}
