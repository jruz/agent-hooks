use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("HOME not set: {0}")]
    HomeNotSet(std::env::VarError),
}

pub fn state_home() -> Result<PathBuf, Error> {
    let xdg_state_home = std::env::var_os("XDG_STATE_HOME");
    match xdg_state_home {
        Some(path) => {
            let path = PathBuf::from(path);
            Ok(path)
        }
        None => home_state_dir(),
    }
}

fn home_state_dir() -> Result<PathBuf, Error> {
    let home = std::env::var("HOME");
    let home = home.map_err(Error::HomeNotSet)?;
    let path = PathBuf::from(home);
    let local = path.join(".local");
    let state = local.join("state");
    Ok(state)
}
