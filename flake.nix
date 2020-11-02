{
  description = "Flake to build bitte";

  inputs = {
    crystal.url = "github:kreisys/crystal";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    inclusive.url = "github:input-output-hk/nix-inclusive";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, crystal, inclusive, utils, ... }:
    utils.lib.eachSystem [ "x86_64-linux" "x86_64-darwin" ] (system: rec {
      overlay = final: prev: {
        nixos-rebuild = let
          nixos = nixpkgs.lib.nixosSystem {
            inherit system;
            modules = [{ nix.package = prev.nixFlakes; }];
          };
        in nixos.config.system.build.nixos-rebuild;

        terraform-with-plugins = prev.terraform.withPlugins (plugins:
          nixpkgs.lib.attrVals [ "null" "local" "aws" "tls" "sops" "acme" ] plugins);

        inherit (inclusive.lib) inclusive;

        inherit (crystal.legacyPackages.${system})
          crystal shards crystal2nix;

        inherit (prev.callPackage ./. { inherit (final) nixos-rebuild; }) bitte;
      };

      legacyPackages = import nixpkgs {
        inherit system;
        overlays = [ overlay ];
      };

      packages = {
        inherit (legacyPackages) bitte nixos-rebuild nixFlakes sops crystal;
      };

      defaultPackage = legacyPackages.bitte;

      devShell = with self.legacyPackages.${system};
        mkShell {
          buildInputs = [
            nixFlakes
            crystal
            crystal2nix
            shards
            libssh2
            terraform-with-plugins
            cfssl
            sops
            openssl
            pkgconfig
          ];
        };

        hydraJobs = packages;
    });
}
