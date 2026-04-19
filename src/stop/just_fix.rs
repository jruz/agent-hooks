use std::process::Command;

use crate::shared::command::combine_output;

pub fn run() -> Result<(), String> {
    let output = Command::new("just")
        .arg("fix")
        .output()
        .map_err(|err| format!("failed to run `just fix`: {err}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!("just fix failed:\n{}", combine_output(&output)))
    }
}
