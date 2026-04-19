use std::process::Command;

use crate::shared::command::combine_output;

pub fn run() -> Result<(), String> {
    let output = Command::new("slop-detector")
        .output()
        .map_err(|err| format!("failed to run slop-detector: {err}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "slop-detector failed:\n{}",
            combine_output(&output)
        ))
    }
}
