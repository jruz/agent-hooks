use std::process::Command;

use crate::shared::command::combine_output;

pub fn run() -> Result<(), String> {
    let output = Command::new("just")
        .arg("check")
        .output()
        .map_err(|err| format!("failed to run `just check`: {err}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!("just check failed:\n{}", combine_output(&output)))
    }
}
