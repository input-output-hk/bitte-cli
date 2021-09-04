{
  description = "Bitte fläken Sie sich";

  inputs = {
    digga.url = "github:divnix/digga";  # https://digga.divnix.com
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    naersk.url = "github:nrdxp/naersk/git-deps-fix";
  };

  outputs = inputs @ { self, nixpkgs, naersk, digga }:
    with digga.lib; mkFlake {

      inherit self inputs;
      supportedSystems = [ "x86_64-darwin" "x86_64-linux" ];
      channels.nixpkgs = { overlays = [
        naersk.overlay
        ./overlay.nix
      ]; };

      # exported devshell modules
      devshellModules = exportModules (importModules ./devshellModules).modules;

      # provide the pinned build of the bitte package as well as build it with hydra
      outputsBuilder = channels: {
        defaultPackage = channels.nixpkps.bitte;
        hydraJobs = { inherit (channels.nixpkgs) bitte; };
      };

      # the local devshell

      # the channel to use for devshell
      nixos.hostDefaults.channelName = "nixpkgs";
      # externalModules (as opposed to modules) are not re-exported - q.e.d.
      devshell.externalModules = [ ./devshell.nix ];
        
    };
}
