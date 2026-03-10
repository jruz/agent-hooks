# claude-hooks

Rust CLI that runs Claude Code hooks. One subcommand per hook event.

## Usage

```
claude-hooks stop           # Stop hook
claude-hooks pre-tool-use   # PreToolUse hook (reads stdin)
claude-hooks prompt-submit  # UserPromptSubmit hook
```

Configured in `~/.claude/settings.json` as hook commands.

## Architecture

```
src/
  main.rs                # clap CLI, dispatches subcommands, handles output and error logging
  prompt_submit.rs       # PromptSubmit hook — signals zellij with "thinking"
  pre_tool_use.rs        # PreToolUse hook orchestrator
  pre_tool_use/
    input.rs             # stdin JSON deserialization
    block_secrets.rs     # block Read/Grep on .env/.ssh files
    commit_msg_check.rs  # validate commit title format
    bash_rules.rs        # block banned bash command patterns
  stop.rs                # Stop hook orchestrator
  stop/
    claude_trees.rs      # signal claude-trees tab
    zellij.rs            # signal zellij tab renamer
    slop_detector.rs     # run slop-detector binary
    just_fix.rs          # run just fix
    just_check.rs        # run just check
  shared/
    mod.rs               # shared module exports
    response.rs          # JSON response builders (block, etc.)
    zellij.rs            # zellij signal writer (configurable content)
    logger.rs            # structured JSONL file logger
```

## Conventions

- one file per hook action, named after the tool it wraps
- hook modules return raw data, orchestrators handle blocking logic
- `shared/response` builds the JSON that Claude Code expects
- `main.rs` owns all printing (stdout/stderr) and error logging to file
- each module has its own thiserror enum
- event modules use `self_named_module_files` style, `shared/` uses `mod.rs`

## Hook Protocol

Claude Code hooks communicate via JSON on stdout:

- **block**: `{"decision": "block", "reason": "..."}` — stops Claude and shows the reason
- no output — hook passes silently

## Logging

Errors are logged as JSONL to `~/.local/state/claude-hooks/hooks.log` from `main.rs`. If logging fails, errors fall back to stderr. Respects `XDG_STATE_HOME` if set.

## Adding a Hook Action

1. create `src/<event>/<action>.rs` with a `pub fn run()` returning the appropriate result
2. add error enum with thiserror
3. wire it into the event orchestrator (e.g. `stop.rs`)
4. add the error variant to the orchestrator's error enum
