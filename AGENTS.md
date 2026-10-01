# agent-hooks

Rust CLI that runs Claude Code, Codex, and Pi hooks. `pre-tool-use` is agent-specific; `stop` is shared.

## Usage

```sh
agent-hooks pre-tool-use --agent claude # Claude PreToolUse hook (reads stdin)
agent-hooks stop                        # Shared Stop hook
agent-hooks pre-tool-use --agent codex  # Codex PreToolUse hook (reads stdin)
agent-hooks pre-tool-use --agent pi     # Pi PreToolUse hook (reads stdin)
```

Configure each agent's hook settings to pass the matching `--agent` value for `pre-tool-use`. `stop` should call `agent-hooks stop` for Claude, Codex, and Pi.

## Hook Files

- `~/.claude/settings.json`
- `~/.codex/hooks.json`
- `~/.codex/hooks/Stop`
- `~/.codex/hooks/UserPromptSubmit`
- `~/.pi/agent/settings.json`

## Tooling

`cargo` and `bun` are only available inside the project sandbox. Do not run `cargo ...` or `bun ...` directly from the shell.

Use the `just` recipes instead so they execute through the repo's sandbox wrapper:

- `just check`
- `just test`
- `just fix`
- `just run ...`
- `just rust-build`
- `just rust-check`
- `just rust-test`
- `just release-upload <tag>`
- `just release-delete <tag>`

## Architecture

```
src/
  main.rs                # clap CLI, dispatches subcommands, handles output and error logging
  pre_tool_use.rs        # PreToolUse hook orchestrator
  pre_tool_use/
    claude.rs            # Claude-specific payload parsing and response rendering
    codex.rs             # Codex-specific payload parsing and response rendering
    pi.rs                # Pi-specific payload parsing and response rendering
    filters.rs           # shared PreToolUse filter exports
    filters/
      bash.rs            # shared shell policy orchestrator
      bash/
        curl.rs          # inspect curl file inputs and outputs
        paths.rs         # inspect command file operands and redirects
        rules.rs         # executable, flag, and assignment policies
        sandboxed_executable.rs # enforce local sandbox requirements
      commit_msg.rs      # validate parsed git commit messages
      secrets.rs         # block Read/Grep on .env/.ssh files
      shell.rs           # shared shell parser, visitor, and execution context
  stop.rs                # Stop hook orchestrator
  stop/
    slop_detector.rs     # run slop-detector binary
    just_fix.rs          # run just fix
    just_check.rs        # run just check
  shared/
    agent.rs             # shared clap value enum for --agent
    command.rs           # shared command output helpers
    env.rs               # shared environment path helpers
    mod.rs               # shared module exports
    response.rs          # hook JSON response builders
    logger.rs            # structured JSONL file logger
```

## Conventions

- one file per hook action, named after the tool it wraps
- hook modules return raw data, orchestrators handle blocking logic
- provider-specific wire parsing stays at the event edge; shared modules hold policy and stop action logic
- all Bash providers use one shell visitor; policies inspect parsed invocations, assignments, and redirects rather than raw command strings
- sandbox exemptions apply only to sandbox requirements, not command or file restrictions
- SSH and Docker commands retain remote/container execution context; local sandbox requirements are not imposed on their executables
- `shared/response` builds the JSON hook callers expect
- `main.rs` owns all printing (stdout/stderr) and error logging to file
- each module has its own thiserror enum
- event modules use `self_named_module_files` style, `shared/` uses `mod.rs`

## Hook Protocol

Claude Code hooks communicate via JSON on stdout:

- **block**: `{"decision": "block", "reason": "..."}` — stops Claude and shows the reason
- no output — hook passes silently

Codex `pre-tool-use` deny responses use `hookSpecificOutput.permissionDecision = "deny"`.
Codex `pre-tool-use` currently only handles `Bash` events.

Pi `pre-tool-use` accepts Pi's native `toolName` and `input` fields. Blocked
responses use `{ "block": true, "reason": "..." }`.

## Logging

Errors are logged as JSONL to `~/.local/state/agent-hooks/hooks.log` from `main.rs`. If logging fails, errors fall back to stderr. Respects `XDG_STATE_HOME` if set.

## Adding a Hook Action

1. create `src/<event>/<action>.rs` with a `pub fn run()` returning the appropriate result
2. add error enum with thiserror
3. wire it into the event orchestrator (e.g. `stop.rs`)
4. add the error variant to the orchestrator's error enum
