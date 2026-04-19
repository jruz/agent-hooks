use serde::Deserialize;
use thiserror::Error;

use crate::shared::response;

use super::filters;

#[derive(Error, Debug)]
pub enum ClaudeError {
    #[error("failed to parse Claude PreToolUse JSON: {0}")]
    Json(serde_json::Error),
    #[error("failed to serialize Claude PreToolUse response: {0}")]
    Serialize(serde_json::Error),
}

#[derive(Deserialize)]
#[serde(tag = "tool_name", rename_all = "PascalCase")]
enum HookInput {
    Bash {
        tool_input: BashInput,
    },
    Grep {
        tool_input: GrepInput,
    },
    Read {
        tool_input: ReadInput,
    },
    #[serde(other)]
    Unsupported,
}

#[derive(Deserialize)]
struct BashInput {
    command: String,
}

#[derive(Deserialize)]
struct ReadInput {
    file_path: String,
}

#[derive(Deserialize)]
struct GrepInput {
    #[serde(alias = "file_path")]
    path: String,
}

pub fn run(stdin: &str) -> Result<Option<String>, ClaudeError> {
    let input: HookInput = serde_json::from_str(stdin).map_err(ClaudeError::Json)?;
    match input {
        HookInput::Bash { tool_input: bash } => {
            let bash_result = filters::bash::run(&bash.command);
            let result = bash_result.and_then(|()| filters::commit_msg::run(&bash.command));
            build_response(result)
        }
        HookInput::Grep { tool_input: grep } => {
            let result = filters::secrets::run(&grep.path);
            build_response(result)
        }
        HookInput::Read { tool_input: read } => {
            let result = filters::secrets::run(&read.file_path);
            build_response(result)
        }
        HookInput::Unsupported => Ok(None),
    }
}

fn build_response(result: Result<(), String>) -> Result<Option<String>, ClaudeError> {
    match result {
        Ok(()) => Ok(None),
        Err(reason) => response::pre_tool_use_claude_block(reason)
            .map(Some)
            .map_err(ClaudeError::Serialize),
    }
}
