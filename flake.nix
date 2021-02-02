{
  description = "Flake to build bitte";

  inputs = {
    crystal.url = "github:kreisys/crystal";
    utils.url = "github:kreisys/flake-utils";
  };

  outputs = { self, nixpkgs, crystal, utils, ... }:
    utils.lib.simpleFlake {
      inherit nixpkgs;

      name = "bitte";
      systems = [ "x86_64-darwin" "x86_64-linux" ];

      overlay = final: prev: {
        inherit (final.callPackages ./shards { }) shards;

      nixos-rebuild = prev.nixos-rebuild.overrideAttrs (o: {
        src = final.runCommand "nixos-rebuild.sh" {
          inherit (o) src;
        } ''
          substitute $src $out \
          --replace systemctl false
        '';
      });

      bitte = final.callPackage ./package.nix { };
    };

      preOverlays = [ crystal ];

      packages = { bitte }: {
        inherit bitte;
        defaultPackage = bitte;
      };

      shell = { mkShell, pkgs }:
        mkShell {
          buildInputs = with pkgs; [
            nixFlakes
            pkgs.crystal
            crystal2nix
            shards
            libssh2
            cfssl
            sops
            openssl
            pkgconfig
          ];

          nobuildPhase = "touch $out";
        };
    };
}
