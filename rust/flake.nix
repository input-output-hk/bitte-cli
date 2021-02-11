{
  description = "Bitte fläken Sie sich";

  inputs = {
    utils.url = "github:kreisys/flake-utils";
    devshell.url = "github:numtide/devshell";
    devshell.inputs.nixpkgs.follows = "nixpkgs";
    naersk.url = "github:nmattia/naersk";
    naersk.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, devshell, nixpkgs, utils, naersk }:
    utils.lib.simpleFlake {
      inherit nixpkgs;

      systems = [ "x86_64-linux" "x86_64-darwin" ];

      preOverlays = [
        devshell
        naersk
      ];

      overlay = final: prev: {
        bitte = final.naersk.buildPackage {
          pname = "bitte";
          root = ../.;
        };
      };

      packages = { bitte }: {
        inherit bitte;
        defaultPackage = bitte;
      };

    };
}
