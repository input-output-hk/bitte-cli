{
  description = "Bitte fläken Sie sich";

  inputs = {
    utils.url = "github:kreisys/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    naersk.url = "github:nmattia/naersk";
  };

  outputs = { self, nixpkgs, naersk, utils, ... }:
    utils.lib.simpleFlake {
      inherit nixpkgs;

      systems = [ "x86_64-darwin" "x86_64-linux" ];

      preOverlays = [ naersk ];

      overlay = final: prev: {
        nixos-rebuild = prev.nixos-rebuild.overrideAttrs (o: {
          src = prev.runCommand "nixos-rebuild.sh" { inherit (o) src; } ''
            substitute $src $out \
            --replace systemctl false
          '';
        });

        bitte = with builtins;
          prev.naersk.buildPackage {
            # Without this we end up with a drv called `rust-workspace-unknown`
            # which makes `nix run` try to execute a bin with that name.
            inherit ((fromTOML (readFile ./cli/Cargo.toml)).package)
              name version;
            root = self;
            buildInputs = with prev; [ pkg-config openssl zlib ];

            overrideMain = _: {
              postInstall = ''
                mkdir -p "$out/share/"{bash-completion/completions,fish/vendor_completions.d,zsh/site-functions}

                echo "generate completion scripts for iogo"
                $out/bin/iogo completions bash > "$out/share/bash-completion/completions/iogo"
                $out/bin/iogo completions fish > "$out/share/fish/vendor_completions.d/iogo.fish"
                $out/bin/iogo completions zsh >  "$out/share/zsh/site-functions/_iogo"
              '';
            };
          };
      };

      packages = { bitte, nixos-rebuild, ... }: {
        defaultPackage = bitte;
        inherit bitte nixos-rebuild;
      };

      hydraJobs = { bitte, nixos-rebuild, ... }@ps: ps;

      devShell = { mkShell, pkgs, ... }:
        mkShell {
          RUST_BACKTRACE = "1";
          RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;

          buildInputs = with pkgs; [
            cfssl
            sops
            openssl
            zlib
            pkg-config
            rust-analyzer
            cargo
            clippy
            rls
            rustc
            rustfmt
          ];
        };
    };
}
