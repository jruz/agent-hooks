use super::super::shell;

const SANDBOX_REQUIRED_EXECUTABLES: [&str; 27] = [
    "bash", "bun", "cargo", "chroot", "dash", "deno", "doas", "fish", "ksh", "nice", "node",
    "nohup", "npm", "npx", "nu", "nush", "parallel", "pnpm", "python", "python3", "setsid", "sudo",
    "sh", "time", "timeout", "xargs", "zsh",
];

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(
        "Must be prefixed by `sandbox` (e.g. `sandbox cargo build`). Use `just` recipes when available."
    )]
    Unsandboxed,
}

pub fn run(executable: &str, sandbox: shell::Sandbox) -> Result<(), Error> {
    let name = shell::executable_name(executable);
    let requires_sandbox = name == "sandbox" && executable != "sandbox"
        || SANDBOX_REQUIRED_EXECUTABLES.contains(&name);
    if requires_sandbox && sandbox == shell::Sandbox::Outside {
        Err(Error::Unsandboxed)
    } else {
        Ok(())
    }
}
