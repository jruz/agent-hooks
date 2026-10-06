{
  description = "Rust CLI that runs Claude Code hooks";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs?ref=nixos-26.05";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        inherit (pkgs) lib;
        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        rustToolchain = builtins.fromTOML (builtins.readFile ./rust-toolchain.toml);
        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        revision = self.rev or "dirty";
        commonPackages = with pkgs; [
          curl
          deadnix
          fd
          git
          just
          nixfmt
          nushell
          ripgrep
          statix
          taplo
          woodpecker-cli
          starship
        ];
        corePackages = with pkgs; [
          bash
          coreutils
          findutils
          gnugrep
          gnused
        ];
        sandboxPackages = with pkgs; [
          rust
          bun
          cargo-edit
          cargo-nextest
          cargo-outdated
          cargo-watch
          cargo-zigbuild
          neovim
          zig
        ];
        sandboxPath = lib.makeBinPath (
          commonPackages ++ sandboxPackages ++ corePackages ++ [ pkgs.stdenv.cc ]
        );
        sandbox = pkgs.writeShellScriptBin "sandbox" ''
          set -euo pipefail
          PROJECT_DIR="$(pwd)"

          ARGS=(
            --unshare-user
            --unshare-ipc
            --unshare-uts
            --unshare-cgroup
            --die-with-parent
            --ro-bind /nix /nix
            --ro-bind /etc/resolv.conf /etc/resolv.conf
            --symlink ${pkgs.coreutils}/bin/env /usr/bin/env
            --symlink ${pkgs.bash}/bin/bash /bin/sh
            --tmpfs "$HOME"
            --dir "$HOME/.config"
            --dir "$HOME/.local"
            --dir "$HOME/.local/share"
            --dir "$HOME/.local/state"
            --dir "$HOME/.cache"
            --dir "$HOME/.supermaven"
            --bind "$PROJECT_DIR" "$PROJECT_DIR"
            --bind /tmp /tmp
            --setenv HOME "$HOME"
            --setenv PATH "${sandboxPath}"
            --setenv TERM "''${TERM:-xterm}"
            --setenv TMPDIR /tmp
            --dev /dev
            --proc /proc
          )

          [[ -d /etc/ssl ]] && ARGS+=(--ro-bind /etc/ssl /etc/ssl)
          [[ -d /etc/static ]] && ARGS+=(--ro-bind /etc/static /etc/static)
          [[ -d /run/current-system/sw ]] && ARGS+=(--ro-bind /run/current-system/sw /run/current-system/sw)

          [[ -f "$HOME/.gitconfig" ]] && ARGS+=(--ro-bind "$HOME/.gitconfig" "$HOME/.gitconfig")
          [[ -f "$HOME/.ssh/known_hosts" ]] && ARGS+=(--ro-bind "$HOME/.ssh/known_hosts" "$HOME/.ssh/known_hosts")
          [[ -d "$HOME/.config/nvim" ]] && ARGS+=(--bind "$HOME/.config/nvim" "$HOME/.config/nvim")
          [[ -d "$HOME/.local/share/nvim" ]] && ARGS+=(--bind "$HOME/.local/share/nvim" "$HOME/.local/share/nvim")
          [[ -d "$HOME/.local/state/nvim" ]] && ARGS+=(--bind "$HOME/.local/state/nvim" "$HOME/.local/state/nvim")
          [[ -d "$HOME/.cache/nvim" ]] && ARGS+=(--bind "$HOME/.cache/nvim" "$HOME/.cache/nvim")
          [[ -d "$HOME/.supermaven" ]] && ARGS+=(--bind "$HOME/.supermaven" "$HOME/.supermaven")

          mkdir -p "$HOME/.bun"
          ARGS+=(--bind "$HOME/.bun" "$HOME/.bun")

          mkdir -p "$HOME/.cargo"
          ARGS+=(--bind "$HOME/.cargo" "$HOME/.cargo")

          exec ${pkgs.bubblewrap}/bin/bwrap "''${ARGS[@]}" -- "$@"
        '';
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = cargoToml.package.name;
          inherit (cargoToml.package) version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          REVISION = revision;
          CARGO_HOME = "$TMPDIR";
          doCheck = false;
          buildInputs = pkgs.lib.optionals pkgs.stdenv.isDarwin [ pkgs.libiconv ];
        };

        devShells.default = pkgs.mkShell {
          packages = commonPackages ++ corePackages ++ [ sandbox ];
          shellHook = ''
            echo ""
            echo "  ${cargoToml.package.name} Development Shell"
            echo "  ───────────────────────────"
            echo "  rust   ${rustToolchain.toolchain.channel} (sandbox)"
            echo "  bun    ${pkgs.bun.version} (sandbox)"
            echo ""
          '';
        };
      }
    );
}
