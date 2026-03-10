mod bash_rules;
mod block_secrets;
mod commit_msg_check;
mod input;

use thiserror::Error;

use crate::hook_response;

#[derive(Error, Debug)]
pub enum PreToolUseError {
    #[error("{0}")]
    Input(#[from] input::InputError),
    #[error("{0}")]
    Serialize(#[from] serde_json::Error),
}

pub fn run() -> Result<Option<String>, PreToolUseError> {
    let input = input::read_stdin()?;

    let block_reason = block_secrets::check(&input)
        .and_then(|()| commit_msg_check::check(&input))
        .and_then(|()| bash_rules::check(&input))
        .err();

    match block_reason {
        Some(reason) => Ok(Some(hook_response::block(reason)?)),
        None => Ok(None),
    }
}
