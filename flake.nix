{
  description = "Bitte fläken Sie sich";

  inputs = {
    utils.url = "github:kreisys/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    iogo.url = "github:input-output-hk/bitte-iogo";
    iogo.inputs.nixpkgs.follows = "nixpkgs";
    iogo.inputs.utils.follows = "utils";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, utils, iogo, fenix, ... }:
    utils.lib.simpleFlake {
      inherit nixpkgs;

      systems = [ "x86_64-darwin" "x86_64-linux" ];

      preOverlays = [
        iogo.overlay
        fenix.overlay
      ];

      overlay = let
      in final: prev: {
        bitte = final.callPackage ./package.nix { };
        damon = final.callPackage (import ./pkgs/damon.nix prev.fetchurl) { };
        bitteShell = final.callPackage ./pkgs/bitte-shell.nix { };
      };

      packages = { bitte, damon }: {
        defaultPackage = bitte;
        inherit bitte damon;
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
