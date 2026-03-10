# claude-hooks

Rust CLI that runs Claude Code hooks. One subcommand per hook event.

## Usage

```
claude-hooks stop    # Stop hook
```

Configured in `~/.claude/settings.json` as hook commands.

## Architecture

```
src/
  main.rs              # clap CLI, dispatches subcommands, handles output
  hook_response.rs     # shared JSON response builders (block, etc.)
  stop.rs              # Stop hook orchestrator
  stop/
    claude_trees.rs    # signal claude-trees tab
    zellij.rs          # signal zellij tab renamer
    slop_detector.rs   # run slop-detector binary
    just_fix.rs        # run just fix
    just_check.rs      # run just check
```

## Conventions

- one file per hook action, named after the tool it wraps
- hook modules return raw data, orchestrator (`stop.rs`) handles blocking logic
- `hook_response` builds the JSON that Claude Code expects
- `main.rs` owns all printing (stdout/stderr), modules return values
- each module has its own thiserror enum
- `stop.rs` uses `self_named_module_files` style instead of `mod.rs`

## Hook Protocol

Claude Code hooks communicate via JSON on stdout:

- **block**: `{"decision": "block", "reason": "..."}` — stops Claude and shows the reason
- no output — hook passes silently

## Adding a Hook Action

1. create `src/<event>/<action>.rs` with a `pub fn run()` returning the appropriate result
2. add error enum with thiserror
3. wire it into the event orchestrator (e.g. `stop.rs`)
4. add the error variant to the orchestrator's error enum
