{
  description = "Bitte fläken Sie sich";

  inputs = {
    utils.url = "github:numtide/flake-utils";
    devshell.url = "github:numtide/devshell";
    treefmt.url = "github:numtide/treefmt";
    treefmt.inputs.nixpkgs.follows = "nixpkgs";

    iogo.url = "github:input-output-hk/bitte-iogo";
    iogo.inputs.devshell.follows = "devshell";
    iogo.inputs.nixpkgs.follows = "nixpkgs";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "nixpkgs/nixos-unstable";

    ragenix.url = "github:input-output-hk/ragenix";
  };

  outputs =
    { self, nixpkgs, utils, iogo, fenix, devshell, treefmt, ... }@inputs:
    let
      overlays = [ fenix.overlay devshell.overlay ];

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
