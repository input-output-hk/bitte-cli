{
  description = "Bitte fläken Sie sich";

  inputs = {
    utils.url = "github:kreisys/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    naersk.url = "github:nrdxp/naersk/git-deps-fix";
    naersk.inputs.nixpkgs.follows = "nixpkgs";
    iogo.url = "github:manveru/bitte-iogo";
    iogo.inputs.nixpkgs.follows = "nixpkgs";
    iogo.inputs.utils.follows = "utils";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, naersk, utils, iogo, fenix, ... }:
    utils.lib.simpleFlake {
      inherit nixpkgs;

      systems = [ "x86_64-darwin" "x86_64-linux" ];

      preOverlays = [
        naersk
        iogo.overlay
        fenix.overlay
        (final: prev: {
          naersk = prev.naersk.override {
            inherit (fenix.packages.${prev.system}.stable) cargo rustc;
          };
        })
      ];

      overlay = final: prev: {
        bitte = final.callPackage ./package.nix { };
        bitteShellCompat = final.callPackage ./pkgs/bitte-shell.nix { };
      };

      packages = { bitte }: {
        defaultPackage = bitte;
        inherit bitte;
      };

      hydraJobs = { bitte }@ps: ps;

      extraOutputs = {
        pkgs = import nixpkgs {
          system = "x86_64-linux";
          overlays = [ fenix.overlay ];
        };
      };

      devShell = { mkShell, pkgs, stdenv, lib, darwin }:
        mkShell {
          RUST_BACKTRACE = "1";
          RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;

          buildInputs = with pkgs;
            [
              lld
              cfssl
              sops
              openssl
              zlib
              pkg-config
              (pkgs.fenix.stable.withComponents [
                "cargo"
                "clippy"
                "rust-src"
                "rustc"
                "rustfmt"
              ])
              rust-analyzer-nightly
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
