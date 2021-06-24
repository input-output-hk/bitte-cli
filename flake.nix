{
  description = "Bitte fläken Sie sich";

  inputs = {
    utils.url = "github:kreisys/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    naersk.url = "github:nmattia/naersk";
  };

  outputs = { self, nixpkgs, naersk, utils, ... }:
    utils.lib.simpleFlake {
      inherit nixpkgs;

      systems = [ "x86_64-darwin" "x86_64-linux" ];

      preOverlays = [ naersk ];

      overlay = final: prev: {
        bitte = final.callPackage ./package.nix {};
      };

      packages = { bitte, ... }: {
        defaultPackage = bitte;
        inherit bitte;
      };

      hydraJobs = { bitte, ... }@ps: ps;

      devShell = { mkShell, pkgs, ... }:
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
            rls
            rustc
            rustfmt
          ];
        };
    };
}
