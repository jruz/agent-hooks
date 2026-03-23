{
  description = "DevShell";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-25.11";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = builtins.fromTOML (builtins.readFile ./rust-toolchain.toml);
        rustVersion = rustToolchain.toolchain.channel;
        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        commonPackages = with pkgs; [
          bun
          cargo-edit
          cargo-nextest
          cargo-outdated
          cargo-watch
          just
          rust
          woodpecker-cli
        ];
        darwinPackages = with pkgs; [ libiconv ];
        linuxPackages = with pkgs; [ glibc glibcLocales ];
        platformPackages =
          if pkgs.stdenv.isDarwin then
            commonPackages ++ darwinPackages
          else
            commonPackages ++ linuxPackages;
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = platformPackages;

          shellHook = ''
            echo ""
            echo "  Rust Development Shell"
            echo "  ───────────────────────────"
            echo "  Rust:        ${rustVersion}"
            echo "  Bun:         ${pkgs.bun.version}"
            echo "  Woodpecker:  ${pkgs.woodpecker-cli.version}"
            echo ""
          '' + pkgs.lib.optionalString pkgs.stdenv.isLinux ''
            export LOCALE_ARCHIVE="${pkgs.glibcLocales}/lib/locale/locale-archive"
          '';
        };
      });
}
