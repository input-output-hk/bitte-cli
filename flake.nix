{
  description = "Bitte fläken Sie sich";

  inputs = {
    utils.follows = "iogo/utils";
    devshell.url = "github:numtide/devshell";
    treefmt.url = "github:numtide/treefmt";
    treefmt.inputs.nixpkgs.follows = "nixpkgs";

    nixpkgs.follows = "fenix/nixpkgs";
    iogo.url = "github:input-output-hk/bitte-iogo";
    iogo.inputs.devshell.follows = "devshell";
    iogo.inputs.nixpkgs.follows = "nixpkgs";
    fenix.url = "github:nix-community/fenix";
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
        bitte = final.callPackage ./package.nix { inherit toolchain self; };
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

      toolchain = "stable";

    in
    utils.lib.eachSystem [ "x86_64-darwin" "x86_64-linux" ]
      (system:
        let
          legacyPackages = pkgsForSystem system;
          rustPkg = legacyPackages.fenix.${toolchain}.withComponents [
            "cargo"
            "clippy"
            "rust-src"
            "rustc"
            "rustfmt"
          ];
        in
        rec
        {
          inherit legacyPackages;

          packages = {
            inherit (legacyPackages) bitte damon;
          };
          defaultPackage = legacyPackages.bitte;
          devShell = with legacyPackages; mkShell {
            RUST_BACKTRACE = "1";
            RUST_SRC_PATH = "${rustPkg}/lib/rustlib/src/rust/library";

            buildInputs = [
              legacyPackages.treefmt
              shfmt
              nodePackages.prettier
              cfssl
              sops
              openssl
              zlib
              pkg-config
              rustPkg
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
