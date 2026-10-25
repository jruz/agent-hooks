install:
    sandbox cargo install \
        --path . \
        --locked \
        --force \
        --root target/dist
    install \
        --directory \
        --mode=755 \
        "$HOME/.cargo/bin"
    install \
        --mode=755 \
        target/dist/bin/* \
        "$HOME/.cargo/bin"

check:
    just nix-check
    just woodpecker-check
    just rust-check

test:
    just rust-test
    just bun-test

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

release-bump VERSION:
    nu scripts/release-bump.nu {{ VERSION }}

release-build TARGET:
    nu scripts/release-build.nu {{ TARGET }}

release-build-all:
    nu scripts/release-build.nu all

release-upload TAG:
    sandbox bun scripts/release-upload.ts {{ TAG }}

release-delete TAG:
    git tag --delete {{ TAG }}
    sandbox bun scripts/release-delete.ts {{ TAG }}

bun-test:
    sandbox bun test scripts

sandbox-test:
    sandbox bun scripts/sandbox-test.ts
