use super::super::shell;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Shell(#[from] shell::Error),
}

pub fn run(command: &str) -> Result<(), Error> {
    shell::run(command)?;
    Ok(())
}
