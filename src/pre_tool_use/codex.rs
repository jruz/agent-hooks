use serde::Deserialize;
use thiserror::Error;

use crate::shared::response;

use super::filters;

#[derive(Error, Debug)]
pub enum CodexError {
    #[error("failed to parse Codex PreToolUse JSON: {0}")]
    Json(serde_json::Error),
    #[error("failed to serialize Codex PreToolUse response: {0}")]
    Serialize(serde_json::Error),
}

#[derive(Deserialize)]
#[serde(tag = "tool_name", rename_all = "PascalCase")]
enum HookInput {
    Bash {
        tool_input: BashInput,
    },
    #[serde(other)]
    Unsupported,
}

#[derive(Deserialize)]
struct BashInput {
    command: String,
}

pub fn run(stdin: &str) -> Result<Option<String>, CodexError> {
    let input: HookInput = serde_json::from_str(stdin).map_err(CodexError::Json)?;
    match input {
        HookInput::Bash { tool_input: bash } => {
            let result = filters::bash::run(&bash.command);
            build_response(result)
        }
        HookInput::Unsupported => Ok(None),
    }
}

fn build_response(result: Result<(), String>) -> Result<Option<String>, CodexError> {
    match result {
        Ok(()) => Ok(None),
        Err(reason) => response::pre_tool_use_codex_block(reason)
            .map(Some)
            .map_err(CodexError::Serialize),
    }
}
