{
  description = "nexum";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
      {
        devShells.default = with pkgs; mkShell {
          buildInputs = [
            # System dependencies
            openssl
            pcsclite
            pkg-config

            # Rust toolchain with WASM target
            ((rust-bin.selectLatestNightlyWith
              (toolchain: toolchain.default)).override
              { targets = [ "wasm32-unknown-unknown" ]; }) # or `toolchain.minimal`

            # Development tools
            wasm-pack      # WASM build tool
            trunk          # WASM web app bundler
            just           # Task runner
            nodePackages.web-ext  # Browser extension development
          ];
        };

        packages.default = nixpkgs.legacyPackages.x86_64-linux.hello;
      }
    );
}
