{
  description = "Bitte fläken Sie sich";

  inputs = {
    utils.url = "github:kreisys/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    devshell.url = "github:numtide/devshell";
    devshell.inputs.nixpkgs.follows = "nixpkgs";
    rust.url = "github:input-output-hk/rust.nix/work";
    rust.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, devshell, rust, utils, ... }:
    utils.lib.simpleFlake {
      inherit nixpkgs;

      systems = [ "x86_64-darwin" "x86_64-linux" ];

      preOverlays = [
        rust
        devshell
      ];

      overlay = final: prev: {
        nixos-rebuild = prev.nixos-rebuild.overrideAttrs (o: {
          src = final.runCommand "nixos-rebuild.sh" { inherit (o) src; } ''
            substitute $src $out \
            --replace systemctl false
          '';
        });

        bitte = with builtins;
          final.rust-nix.buildPackage {
            # Without this we end up with a drv called `rust-workspace-unknown`
            # which makes `nix run` try to execute a bin with that name.
            inherit ((fromTOML (readFile ./cli/Cargo.toml)).package)
              name version;
            root = self;
            buildInputs = with final; [ pkg-config openssl ];
          };
      };

      packages = { bitte }: {
        defaultPackage = bitte;
      };

      hydraJobs = { bitte }@ps: ps;

      devShell = { devshell, pkgs }:
        devshell.mkShell {
          env = with nixpkgs.lib; mapAttrsToList nameValuePair ({
            RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc.outPath;
            RUST_BACKTRACE = "1";
          });

          packages = with pkgs; [
            nixFlakes
            libssh2
            cfssl
            sops
            openssl
            pkg-config

            # Rust
            rustc
            cargo
            (rustracer.overrideAttrs (_: { checkPhase = null; }))
            rust-analyzer
            rustfmt
            clippy
          ];
        };
    };
}
