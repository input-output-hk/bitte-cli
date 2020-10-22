{
  description = "Flake to build bitte";

  inputs = {
    crystal.url = "github:kreisys/crystal";
    nixpkgs.url = "github:Nixos/nixpkgs/nixos-20.09";
    utils.url = "github:kreisys/flake-utils";

    # TODO maybe just a patch instead of pulling a whole 'nother nixpkgs?
    "nixpkgs/nixos-rebuild-no-systemctl".url = "github:kreisys/nixpkgs/nixos-rebuild-no-systemctl";
  };

  outputs = { self, nixpkgs, crystal, utils, ... }: utils.lib.simpleFlake {
    inherit nixpkgs;

    name = "bitte";
    systems = [ "x86_64-darwin" "x86_64-linux" ];

    overlay = final: prev: {
      inherit (final.callPackages ./shards {}) shards;

      nixos-rebuild =
        let
          nixos = nixpkgs.lib.nixosSystem {
            inherit (final) system;
            modules = let toolsModule = "installer/tools/tools.nix"; in
            [{
              disabledModules = [ toolsModule ];
              imports = [ "${self.inputs."nixpkgs/nixos-rebuild-no-systemctl"}/nixos/modules/${toolsModule}" ];
              config.nix.package = prev.nixFlakes;
            }];
          };
        in
        nixos.config.system.build.nixos-rebuild;

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
        crystal
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
