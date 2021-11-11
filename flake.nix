{
  description = "Bitte fläken Sie sich";

  inputs = {
    utils.url = "github:numtide/flake-utils";
    devshell.url = "github:numtide/devshell";
    treefmt.url = "github:numtide/treefmt";

    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    iogo.url = "github:input-output-hk/bitte-iogo";
    iogo.inputs.nixpkgs.follows = "nixpkgs";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, utils, iogo, fenix, devshell, treefmt, ... }:
    let
      overlays = [
        iogo.overlay
        fenix.overlay
        devshell.overlay
        pkgsOverlays
      ];

      pkgsOverlays = final: prev: {
        bitte = final.callPackage ./package.nix { };
        damon = final.callPackage (import ./pkgs/damon.nix prev.fetchurl) { };
        treefmt = treefmt.defaultPackage.${final.system};
        bitteShell = final.callPackage ./pkgs/bitte-shell.nix {
          bitteDevshellModule = self.devshellModules.bitte;
        };
      };

      pkgsForSystem = system: import nixpkgs {
        inherit overlays system;
        config.allowUnfree = true;
      };


    in
    utils.lib.eachSystem [ "x86_64-darwin" "x86_64-linux" ]
      (system: rec {
        legacyPackages = pkgsForSystem system;
        packages = {
          defaultPackage = legacyPackages.bitte;
          inherit (legacyPackages) bitte damon;
        };
        devShell = with legacyPackages; mkShell {
          RUST_BACKTRACE = "1";
          RUST_SRC_PATH = rustPlatform.rustLibSrc;

          buildInputs = [
            legacyPackages.treefmt
            shfmt
            nodePackages.prettier
            cfssl
            sops
            openssl
            zlib
            pkg-config
            (legacyPackages.fenix.stable.withComponents [
              "cargo"
              "clippy"
              "rust-src"
              "rustc"
              "rustfmt"
            ])
            rust-analyzer-nightly
          ] ++ lib.optionals stdenv.isDarwin (
            with darwin; with apple_sdk.frameworks; [
              libiconv
              libresolv
              Libsystem
              SystemConfiguration
              Security
              CoreFoundation
            ]
          );
        };
      }) // {
      overlay = final: prev: (nixpkgs.lib.composeManyExtensions overlays) final prev;
      devshellModules.bitte = import ./devshellModule.nix;
    }; # outputs
}
