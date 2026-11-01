use super::super::shell;

const SANDBOX_REQUIRED_EXECUTABLES: &[&str] = &[
    "bun",
    "bunx",
    "cargo",
    "conda",
    "corepack",
    "deno",
    "elixir",
    "elixirc",
    "erl",
    "erlc",
    "escript",
    "gleam",
    "hatch",
    "iex",
    "mamba",
    "micromamba",
    "mix",
    "node",
    "nodejs",
    "npm",
    "npx",
    "pdm",
    "pip",
    "pipenv",
    "pipx",
    "pnpm",
    "pnpx",
    "pnx",
    "poetry",
    "pypy",
    "python",
    "rebar3",
    "rustc",
    "rustdoc",
    "rustup",
    "uv",
    "uvx",
    "yarn",
    "yarnpkg",
];

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(
        "Must be prefixed by `sandbox` (e.g. `sandbox cargo build`). Use `just` recipes when available."
    )]
    Unsandboxed,
    #[error("Use bare `sandbox`, not an absolute or relative wrapper path.")]
    WrapperPath,
}

pub fn run(executable: &str, sandbox: shell::Sandbox) -> Result<(), Error> {
    let name = shell::executable_name(executable);
    if name == "sandbox" && executable != "sandbox" && sandbox == shell::Sandbox::Outside {
        return Err(Error::WrapperPath);
    }
    let requires_sandbox =
        SANDBOX_REQUIRED_EXECUTABLES.contains(&name) || is_versioned_python_executable(name);
    if requires_sandbox && sandbox == shell::Sandbox::Outside {
        Err(Error::Unsandboxed)
    } else {
        Ok(())
    }
}

fn is_versioned_python_executable(name: &str) -> bool {
    ["python", "pip", "pypy"]
        .iter()
        .filter_map(|prefix| name.strip_prefix(prefix))
        .any(|version| version.split('.').all(is_version_component))
}

fn is_version_component(component: &str) -> bool {
    !component.is_empty()
        && component
            .chars()
            .all(|character| character.is_ascii_digit())
}
