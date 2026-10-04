#[path = "support/hook_json.rs"]
mod hook_json;

const AGENTS: [&str; 3] = ["claude", "codex", "pi"];
const SANDBOX_REASON: Option<&str> = Some(
    "Must be prefixed by `sandbox` (e.g. `sandbox cargo build`). Use `just` recipes when available.",
);
const DEPENDENCY_EXECUTABLES: &[&str] = &[
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
    "pip2",
    "pip3",
    "pip3.14",
    "pipenv",
    "pipx",
    "pnpm",
    "pnpx",
    "pnx",
    "poetry",
    "pypy",
    "pypy3",
    "pypy3.11",
    "python",
    "python2",
    "python2.7",
    "python3",
    "python3.9",
    "python3.14",
    "rebar3",
    "rustc",
    "rustdoc",
    "rustup",
    "uv",
    "uvx",
    "yarn",
    "yarnpkg",
];

fn assert_decision(command: &str, expected: Option<&str>) {
    for agent in AGENTS {
        let input = if agent == "pi" {
            serde_json::json!({"toolName": "bash", "input": {"command": command}})
        } else {
            serde_json::json!({"tool_name": "Bash", "tool_input": {"command": command}})
        };
        let json = input.to_string();
        let output = hook_json::run_pre_tool_use_json(agent, &json);
        assert!(output.status.success(), "{agent}: {command}");
        assert!(
            output.stderr.is_empty(),
            "{agent}: {command}: {:?}",
            output.stderr
        );
        let reason = hook_json::blocked_reason(&output);
        assert_eq!(reason.as_deref(), expected, "{agent}: {command}");
    }
}

fn assert_requires_sandbox(command: &str) {
    assert_decision(command, SANDBOX_REASON);
    let sandboxed = format!("sandbox {command}");
    assert_decision(&sandboxed, None);
}

#[test]
fn dependency_executables_require_sandbox_in_common_invocation_forms() {
    for executable in DEPENDENCY_EXECUTABLES {
        let commands = [
            format!("{executable} --version"),
            format!("/usr/bin/{executable} --version"),
            format!("./bin/{executable} --version"),
            format!("'{executable}' --version"),
            format!("\"{executable}\" --version"),
            format!("MODE=test {executable} --version"),
            format!("env MODE=test {executable} --version"),
            format!("command -- {executable} --version"),
            format!("exec {executable} --version"),
            format!("bash -c '{executable} --version'"),
            format!("sh -c '{executable} --version'"),
            format!("nix-shell --run '{executable} --version'"),
        ];
        for command in commands {
            assert_requires_sandbox(&command);
        }
    }
}

#[test]
fn dependency_execution_subcommands_and_flags_require_sandbox() {
    let commands = [
        "bun install",
        "bun add lodash",
        "bun run build",
        "bun test",
        "bun x eslint .",
        "bun --cwd app install",
        "bunx eslint .",
        "bunx --bun eslint .",
        "bunx --package @angular/cli ng version",
        "bunx -p renovate renovate-config-validator",
        "bunx --no-install eslint .",
        "cargo build",
        "cargo test",
        "cargo run",
        "cargo install ripgrep",
        "cargo +stable build",
        "cargo --manifest-path app/Cargo.toml check",
        "rustc build.rs",
        "rustdoc --test src/lib.rs",
        "rustup update",
        "deno run script.ts",
        "deno task build",
        "deno test",
        "deno install npm:eslint",
        "node script.js",
        "node --eval 'console.log(1)'",
        "nodejs script.js",
        "npm install",
        "npm ci",
        "npm exec -- eslint .",
        "npm --prefix app install",
        "npx --yes eslint .",
        "npx --package eslint eslint .",
        "pnpm install",
        "pnpm run build",
        "pnpm exec eslint .",
        "pnpm dlx eslint .",
        "pnpm --dir app install",
        "pnpx eslint .",
        "pnx eslint .",
        "yarn install",
        "yarn dlx eslint .",
        "yarnpkg install",
        "corepack pnpm install",
        "python script.py",
        "python3 -m pip install requests",
        "python3.14 -m pytest",
        "pypy3 script.py",
        "pip install requests",
        "pip3 install requests",
        "pip3.14 install requests",
        "pipx run ruff check .",
        "pipenv install",
        "poetry install",
        "pdm install",
        "hatch run test",
        "conda install numpy",
        "mamba install numpy",
        "micromamba install numpy",
        "uv sync",
        "uv run pytest",
        "uv pip install requests",
        "uv tool run ruff check .",
        "uvx ruff check .",
        "uvx --from ruff ruff check .",
        "gleam build",
        "gleam test",
        "gleam deps download",
        "gleam add gleam_json",
        "gleam export erlang-shipment",
        "mix deps.get",
        "mix compile",
        "mix test",
        "mix phx.server",
        "mix archive.install hex phx_new",
        "MIX_ENV=test mix test",
        "elixir script.exs",
        "elixirc lib/app.ex",
        "iex -S mix",
        "erl -noshell -s app start",
        "erlc app.erl",
        "escript script.escript",
        "rebar3 compile",
    ];
    for command in commands {
        assert_requires_sandbox(command);
    }
}

#[test]
fn dependency_executables_are_checked_in_shell_pipelines_and_substitutions() {
    for executable in DEPENDENCY_EXECUTABLES {
        let commands = [
            format!("echo ready; {executable} --version"),
            format!("echo ready && {executable} --version"),
            format!("false || {executable} --version"),
            format!("printf input | {executable} --version"),
            format!("echo \"$({executable} --version)\""),
            format!("({executable} --version)"),
        ];
        for command in commands {
            assert_decision(&command, SANDBOX_REASON);
            let sandboxed = format!("sandbox bash -c '{command}'");
            assert_decision(&sandboxed, None);
        }
    }
}

#[test]
fn dependency_names_in_text_and_unrelated_executables_are_allowed() {
    for executable in DEPENDENCY_EXECUTABLES {
        let commands = [
            format!("echo '{executable} install'"),
            format!("rg '{executable}' src"),
            format!("git status # {executable} install"),
            format!("/usr/bin/{executable}-helper --version"),
        ];
        for command in commands {
            assert_decision(&command, None);
        }
    }
    let commands = [
        "python-config --help",
        "python3.14-config --help",
        "python3. --help",
        "python.3 --help",
        "python3..14 --help",
        "pip-tools --help",
        "pipeline --help",
        "pypy-helper --help",
        "uvicorn --help",
        "bunny --help",
        "mixer --help",
        "erlang-ls --help",
        "gleam-lsp --help",
        "mixer --help",
        "erlang-ls --help",
        "just rust-test",
    ];
    for command in commands {
        assert_decision(command, None);
    }
}

#[test]
fn local_sandbox_requirements_do_not_apply_to_remote_or_container_execution() {
    for executable in DEPENDENCY_EXECUTABLES {
        let commands = [
            format!("ssh host {executable} --version"),
            format!("docker exec container {executable} --version"),
        ];
        for command in commands {
            assert_decision(&command, None);
        }
    }
}

#[test]
fn shells_and_generic_launchers_do_not_require_sandbox() {
    let commands = [
        "bash -c 'git status'",
        "chroot /tmp git status",
        "dash -c 'git status'",
        "doas git status",
        "fish -c 'git status'",
        "ksh -c 'git status'",
        "nice git status",
        "nohup git status",
        "nu --commands '^just --show apple-sync-logs-clear | lines | skip 2 | str join (char newline) | nu-check --debug'",
        "/usr/bin/nu -c '^git status'",
        "nush -e '(print 1)'",
        "parallel git status ::: .",
        "setsid git status",
        "sudo git status",
        "sh -c 'git status'",
        "time git status",
        "timeout 30s git status",
        "printf input | xargs rg",
        "zsh -c 'git status'",
    ];
    for command in commands {
        assert_decision(command, None);
    }
}
