mod just_check;
mod just_fix;
mod slop_detector;

use std::path::Path;

pub fn run() -> Result<(), String> {
    slop_detector::run()?;

    if justfile_exists() {
        just_fix::run()?;
        just_check::run()?;
    }

    Ok(())
}

fn justfile_exists() -> bool {
    Path::new("justfile").exists()
}
