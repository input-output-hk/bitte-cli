{
  description = "Flake to build bitte";

  inputs = {
    nixpkgs-crystal.url = "github:manveru/nixpkgs/crystal-0.35";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    inclusive.url = "github:manveru/nix-inclusive";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, nixpkgs-crystal, inclusive, utils, ... }:
    utils.lib.eachDefaultSystem (system: rec {
      overlay = final: prev: {
        nixos-rebuild = let
          nixos = nixpkgs.lib.nixosSystem {
            inherit system;
            modules = [{ nix.package = prev.nixFlakes; }];
          };
        in nixos.config.system.build.nixos-rebuild;

        terraform-with-plugins = prev.terraform.withPlugins (plugins:
          nixpkgs.lib.attrVals [ "null" "local" "aws" "tls" "sops" ] plugins);

        inherit (inclusive.lib) inclusive;

        inherit (nixpkgs-crystal.legacyPackages.${system})
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
          ];
        };
    });
}
