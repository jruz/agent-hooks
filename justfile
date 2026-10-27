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
    just sandbox-check
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
    just rust-machete

rust-machete:
    sandbox cargo machete --with-metadata

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
    gh release view {{ TAG }} \
        || gh release create {{ TAG }} \
            --title {{ TAG }} \
            --verify-tag
    gh release upload {{ TAG }} \
        {{ env("CI_REPO_NAME") }}-* \
        --clobber

release-delete TAG:
    gh release delete {{ TAG }} \
        --cleanup-tag \
        --yes

sandbox-test:
    just sandbox-check

[positional-arguments]
sandbox-check *EXTRA_ALLOWED:
    just sandbox-isolation-check
    just sandbox-allowlist-check "$@"
    just sandbox-tmp-check
    just sandbox-args-check

sandbox-isolation-check:
    sandbox bun scripts/sandbox-isolation-check.ts

[positional-arguments]
sandbox-allowlist-check *EXTRA_ALLOWED:
    sandbox nu scripts/sandbox-isolation-check.nu "$@"

sandbox-tmp-check:
    nu scripts/sandbox-tmp-check.nu

sandbox-args-check:
    nu scripts/sandbox-args-check.nu
