{
  description = "Bitte fläken Sie sich";

  inputs = {
    utils.url = "github:numtide/flake-utils";
    devshell.url = "github:numtide/devshell";
    nix.url = "github:nixos/nix/2.6.0"; # might need ditribute fixed versions

    nixpkgs.url = "github:nixos/nixpkgs/efeefb2af1469a5d1f0ae7ca8f0dfd9bb87d5cfb";
    treefmt-nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    iogo.url = "github:input-output-hk/bitte-iogo";
    fenix.url = "github:nix-community/fenix";
  };

  outputs = { self, nixpkgs, utils, iogo, fenix, devshell, treefmt-nixpkgs,... }@inputs:
    let
      overlays = [
        iogo.overlay
        fenix.overlay
        devshell.overlay
        pkgsOverlays
      ];

      pkgsOverlays = final: prev: {
        bitte = final.callPackage ./cli/package.nix { inherit toolchain; };
        inherit (treefmt-nixpkgs.legacyPackages."${final.system}") treefmt;
        bitteShell = final.callPackage ./shell/pkgs/bitte-shell.nix {
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
          rustPkg = legacyPackages.fenix."${toolchain}".withComponents [
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
            inherit (legacyPackages) bitte;
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
      overlay = nixpkgs.lib.composeManyExtensions overlays;
      devshellModules.bitte = import ./shell/devshellModule.nix inputs;
    }; # outputs
}
