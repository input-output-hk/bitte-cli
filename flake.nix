{
  description = "Bitte fläken Sie sich";

  inputs = {
    utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs, utils, fenix, ... }@inputs:
    let
      overlays = [ fenix.overlay ];

      pkgsForSystem = system:
        import nixpkgs {
          inherit overlays system;
          config.allowUnfree = true;
        };

      toolchain = "stable";

    in utils.lib.eachSystem [ "x86_64-darwin" "x86_64-linux" ] (system:
      let
        legacyPackages = pkgsForSystem system;
        rustPkg = legacyPackages.fenix."${toolchain}".withComponents [
          "cargo"
          "clippy"
          "rust-src"
          "rustc"
          "rustfmt"
        ];
      in rec {
        inherit legacyPackages;

        packages = {
          bitte =
            legacyPackages.callPackage ./cli/package.nix { inherit toolchain; };
        };
        defaultPackage = legacyPackages.bitte;
        devShell = with legacyPackages;
          mkShell {
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
      }) // {
        devshellModules.bitte = import ./shell/devshellModule.nix inputs;
      }; # outputs
}
