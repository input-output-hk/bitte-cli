{
  description = "Bitte fläken Sie sich";

  inputs = {
    utils.url = "github:kreisys/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust.url = "github:input-output-hk/rust.nix/work";
    rust.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, rust, utils, ... }:
    utils.lib.simpleFlake {
      inherit nixpkgs;

      systems = [ "x86_64-darwin" "x86_64-linux" ];

      preOverlays = [ rust ];

      overlay = final: prev: {
        nixos-rebuild = prev.nixos-rebuild.overrideAttrs (o: {
          src = prev.runCommand "nixos-rebuild.sh" { inherit (o) src; } ''
            substitute $src $out \
            --replace systemctl false
          '';
        });

        bitte = with builtins;
          prev.rust-nix.buildPackage {
            # Without this we end up with a drv called `rust-workspace-unknown`
            # which makes `nix run` try to execute a bin with that name.
            inherit ((fromTOML (readFile ./cli/Cargo.toml)).package)
              name version;
            root = self;
            buildInputs = with prev; [ pkg-config openssl zlib ];
          };
      };

      packages = { bitte, nixos-rebuild }: { defaultPackage = bitte; inherit nixos-rebuild; };

      hydraJobs = { bitte, nixos-rebuild }@ps: ps;

      devShell = { mkShell, pkgs }:
        mkShell {
          RUST_BACKTRACE = "1";
          RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;

          buildInputs = with pkgs; [
            cfssl
            sops
            openssl
            zlib
            pkg-config
            rust-analyzer
            cargo
            clippy
            rustc
            rustfmt
          ];
        };
    };
}
