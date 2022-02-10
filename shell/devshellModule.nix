inputs: { lib, config, pkgs, extraModulesPath, ... }:
let

  # TODO: remove
  # backport form 21.11
  # bitte pins bitte-cli to 21.11
  # but use of overlays destroy it all
  writeShellApplication =
    { name
    , text
    , runtimeInputs ? [ ]
    , checkPhase ? null
    }:
    pkgs.writeTextFile {
      inherit name;
      executable = true;
      destination = "/bin/${name}";
      text = ''
        #!${pkgs.runtimeShell}
        set -o errexit
        set -o nounset
        set -o pipefail

        export PATH="${lib.makeBinPath runtimeInputs}:$PATH"

        ${text}
      '';

      checkPhase =
        if checkPhase == null then ''
          runHook preCheck
          ${pkgs.stdenv.shell} -n $out/bin/${name}
          ${pkgs.shellcheck}/bin/shellcheck $out/bin/${name}
          runHook postCheck
        ''
        else checkPhase;

      # backwards incompatible:
      # meta.mainProgram = name;
    };


  mkStringOptionType = description: lib.mkOption {
    inherit description;
    type = lib.types.str;
  };

  mkOptionalStringOptionType = description: lib.mkOption {
    inherit description;
    type = lib.types.nullOr lib.types.str;
    default = null;
  };

  mkAttrsOptionType = description: lib.mkOption {
    inherit description;
    type = lib.types.attrs;
  };

  mkProviderOptionType = description: lib.mkOption {
    inherit description;
    type = lib.types.enum [ "AWS" ];
  };

  cfg = config.bitte;

  asgRegionString = asg:
    let
      asgRegions = lib.attrValues
        (lib.mapAttrs (_: v: v.region) asg);
    in
    lib.strings.replaceStrings [ " " ] [ ":" ]
      (toString asgRegions);

  withCategory = category: attrset: attrset // { inherit category; };
  infra = withCategory "infra";
  app = withCategory "app";
  utils = withCategory "utils";

in
{
  _file = ./devshellModule.nix;

  options.bitte = {
    cluster = mkStringOptionType "Name of the cluster";
    domain = mkStringOptionType "Cluster root domain";
    namespace = mkOptionalStringOptionType "Cluster main nomad namespace";
    cert = mkOptionalStringOptionType "Certificate to authenticate with nomad, vault & consul";
    provider = mkProviderOptionType "Infrastructure provider";

    aws_region = mkStringOptionType "AWS infrastructure region";
    aws_profile = mkStringOptionType "AWS authentication profile";
    aws_autoscaling_groups = mkAttrsOptionType "AWS auto scaling groups";
  };

  imports = [ "${extraModulesPath}/git/hooks.nix" ];

  config = {
    # tempfix: remove when merged https://github.com/numtide/devshell/pull/123
    devshell.startup.load_profiles = lib.mkForce (lib.noDepEntry "");

    name = cfg.cluster;
    git.hooks = {
      enable = true;
      pre-commit.text = builtins.readFile ./pre-commit.sh;
    };

    commands = [
      (infra {
        package = writeShellApplication {
          name = "diff-against-bitte-commit";
          runtimeInputs = [
            pkgs.nix-diff
            inputs.nix.packages.${pkgs.system}.nix
          ];
          text = builtins.readFile ./diff-against-bitte-commit.sh;
        };
        help = "What changes with bitte commit XYZ";
      })
      (infra { package = pkgs.bitte; })
      (infra { package = pkgs.sops; })
      (infra { package = pkgs.ragenix; })
      (infra { package = pkgs.vault-bin; name = "vault"; })
      (infra { package = pkgs.consul; })
      (infra { package = pkgs.awscli; name = "aws"; })
      (app { package = pkgs.iogo; })
      (app { package = pkgs.nomad; })
      (utils { package = pkgs.bitwarden-cli; name = "bw"; })
      (utils { package = pkgs.jq; })
      (utils { package = pkgs.ijq; })
      (utils { package = pkgs.fx; name = "fx"; })
      (utils { package = pkgs.curlie; })
      (utils { package = pkgs.treefmt; })
      (utils { package = pkgs.go-jira; name = "jira"; })
      (utils { package = pkgs.pwgen; })
    ];

    packages = with pkgs;
      [
        cfssl
        consul-template
        openssl
        python38Packages.pyhcl
        scaler-guard
        vault-bin
        nix-diff

        # treefmt deps
        nixfmt
        nixpkgs-fmt
        nodePackages.prettier
        shfmt

        # pre-commit deps
        editorconfig-checker
        # treefmt -already captured below
      ];

    env = [
      { name = "BITTE_CLUSTER"; value = cfg.cluster; }
      { name = "BITTE_DOMAIN"; value = cfg.domain; }
      { name = "BITTE_PROVIDER"; value = cfg.provider; }
      { name = "VAULT_ADDR"; value = "https://vault.${cfg.domain}"; }
      { name = "NOMAD_ADDR"; value = "https://nomad.${cfg.domain}"; }
      { name = "CONSUL_HTTP_ADDR"; value = "https://consul.${cfg.domain}"; }
    ] ++ (lib.optionals (cfg.namespace != null)) [
      { name = "NOMAD_NAMESPACE"; value = cfg.namespace; }
    ] ++ (lib.optionals (cfg.cert != null)) [
      { name = "CONSUL_CACERT"; value = cfg.cert; }
      { name = "VAULT_CACERT"; value = cfg.cert; }
      { name = "NOMAD_CACERT"; value = cfg.cert; }
    ] ++ (lib.optionals (cfg.provider == "AWS")) [
      { name = "AWS_PROFILE"; value = cfg.aws_profile; }
      { name = "AWS_DEFAULT_REGION"; value = cfg.aws_region; }
      { name = "AWS_ASG_REGIONS"; value = asgRegionString cfg.aws_autoscaling_groups; }
    ];
  };

}
