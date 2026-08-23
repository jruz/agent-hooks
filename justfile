install:
    sandbox cargo install --path . --force

check:
    just nix-check
    just woodpecker-check
    just rust-check

test:
    just rust-test

run *ARGS='':
    just rust-run {{ARGS}}

nvim *ARGS='.':
    sandbox nvim {{ARGS}}

logs:
    nvim ~/.local/state/agent-hooks/hooks.log

watch:
    just rust-watch

fix:
    just nix-fix
    just rust-fix

# ──────────────────────────────── nix ────────────────────────────────

nix-check:
    fd \
        --extension nix \
        --exec nixfmt --check
    statix check .
    deadnix --fail .

nix-fix:
    fd \
        --extension nix \
        --exec nixfmt
    statix fix .
    deadnix --edit .

# ─────────────────────────────── rust ───────────────────────────────

rust-build:
    sandbox cargo build --all-targets

rust-cargo-check:
    sandbox cargo check --all-targets

rust-check:
    just rust-cargo-check
    just rust-fmt-check
    just rust-clippy-check

rust-clippy-check *ARGS='':
    sandbox cargo clippy --all-targets {{ARGS}}

rust-clippy-fix *ARGS='':
    sandbox cargo clippy \
        --fix \
        --all-targets \
        --allow-dirty \
        {{ARGS}}

rust-fix:
    just rust-fmt-fix
    just rust-clippy-fix

rust-fmt-check:
    sandbox cargo fmt --all -- --check

rust-fmt-fix:
    sandbox cargo fmt --all

rust-outdated:
    sandbox cargo outdated

rust-test *FILTER='':
    sandbox cargo nextest run \
        --no-tests=pass \
        {{FILTER}}

rust-run *ARGS='':
    sandbox cargo run -- {{ARGS}}

rust-watch:
    sandbox cargo watch --exec run

rust-upgrade:
    sandbox cargo upgrade --incompatible
    sandbox cargo update

# ────────────────────────────── woodpecker ──────────────────────────────

woodpecker-check:
    woodpecker-cli lint \
        --strict \
        .woodpecker/

# ─────────────────────────────── release ───────────────────────────────

release-build-linux:
    sandbox scripts/release-build.sh unknown-linux-gnu x86_64
    sandbox scripts/release-build.sh unknown-linux-gnu aarch64

release-build-macos:
    sandbox scripts/release-build.sh apple-darwin x86_64
    sandbox scripts/release-build.sh apple-darwin aarch64

release-upload TAG:
    sandbox bun scripts/release-upload.ts {{ TAG }}

release-delete TAG:
    sandbox bun scripts/release-delete.ts {{ TAG }}

sandbox-test:
    sandbox bun scripts/sandbox-test.ts
