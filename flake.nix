{
  description = "Flake to build bitte";

  inputs = {
    crystal.url = "github:kreisys/crystal";
    nixpkgs.follows = "crystal/nixpkgs";
    inclusive.url = "github:input-output-hk/nix-inclusive";
    utils.url = "github:numtide/flake-utils";

    # TODO maybe just a patch instead of pulling a whole 'nother nixpkgs?
    "nixpkgs/nixos-rebuild-no-systemctl".url = "github:kreisys/nixpkgs/nixos-rebuild-no-systemctl";
  };

  outputs = { self, nixpkgs, inclusive, utils, ... }@inputs:
    utils.lib.eachSystem [ "x86_64-linux" "x86_64-darwin" ] (system: rec {
      overlay = final: prev: {
        inherit (final.crystal-flake) crystal crystal2nix;
        shards = final.callPackage ./shards {};
        nixos-rebuild =
          let
            nixos = nixpkgs.lib.nixosSystem {
              inherit system;
              modules = let toolsModule = "installer/tools/tools.nix"; in
                [{
                  disabledModules = [ toolsModule ];
                  imports = [ "${self.inputs."nixpkgs/nixos-rebuild-no-systemctl"}/nixos/modules/${toolsModule}" ];
                  config.nix.package = prev.nixFlakes;
                }];
            };
          in
          nixos.config.system.build.nixos-rebuild;

        terraform-with-plugins = prev.terraform.withPlugins (plugins:
          nixpkgs.lib.attrVals [ "null" "local" "aws" "tls" "sops" "acme" ] plugins);

        bitte = rec {
          cli = final.callPackage ./package.nix { };
          defaultPackage = cli;

          devShell = with final;
            mkShell {
              nobildPhase = "true";
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

              nobuildPhase = "touch $out";
            };
        };

        inherit (inclusive.lib) inclusive;

        inherit (inputs.crystal.legacyPackages.${system})
          crystal shards crystal2nix;

        inherit (prev.callPackage ./. { inherit (final) nixos-rebuild; }) bitte;
      };

      simpleFlake = utils.lib.simpleFlake {
        inherit name systems overlay self nixpkgs;
        preOverlays = [ crystal.overlay ];
      };

    in
    simpleFlake // {
      inherit overlay;
      hydraJobs = self.packages;
      defaultApp = builtins.mapAttrs
        (_: drv:
          utils.lib.mkApp {
            inherit drv;
          }
        ) self.defaultPackage;
    };
}
