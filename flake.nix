{
  description = "Bitte fläken Sie sich";

  inputs = {
    utils.url = "github:kreisys/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    naersk.url = "github:nrdxp/naersk/git-deps-fix";
    naersk.inputs.nixpkgs.follows = "nixpkgs";
    iogo.url = "github:input-output-hk/bitte-iogo";
    iogo.inputs.nixpkgs.follows = "nixpkgs";
    iogo.inputs.utils.follows = "utils";
  };

  outputs = { self, nixpkgs, naersk, utils, iogo, ... }:
    utils.lib.simpleFlake {
      inherit nixpkgs;

      systems = [ "x86_64-darwin" "x86_64-linux" ];

      preOverlays = [ naersk iogo.overlay ];

      overlay = final: prev: {
        bitte = final.callPackage ./package.nix { };
        bitteShellCompat = final.callPackage ./pkgs/bitte-shell.nix { };
      };

      packages = { bitte }: {
        defaultPackage = bitte;
        inherit bitte;
      };

      hydraJobs = { bitte }@ps: ps;

      devShell = { mkShell, pkgs, stdenv, lib, darwin }:
        mkShell {
          RUST_BACKTRACE = "1";
          RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;

          buildInputs = with pkgs;
            [
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
            ] ++ lib.optionals stdenv.isDarwin (with darwin;
              with apple_sdk.frameworks; [
                libiconv
                libresolv
                Libsystem
                SystemConfiguration
                Security
                CoreFoundation
              ]);
        };
    };
}
