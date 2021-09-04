{ lib, config, pkgs, ... }:
with lib;
let
  cfg = config.bitte.cli;

  # Execute this script to load the bitte credentials
  load-credentials = pkgs.writeShellScriptBin "load-credentials" ''
    set -euo pipefail
    shopt -s nullglob

    export PATH=${pkgs.coreutils}/bin:${pkgs.jq}/bin:${pkgs.vault}/bin:${pkgs.consul}/bin:${pkgs.nomad}/bin:$PATH

    log() {
      echo "[bitte.cli] $*" >&2
    }

    is_admin () {
      vault token lookup -format json \
        | jq -e -r '.data.policies | any(. == "admin")' \
        &> /dev/null
    }

    github_token ()  {
      awk '/github.com/ { print $6; exit }' ~/.netrc
    }

    cache="''${XDG_CACHE_HOME:-.direnv}/bitte/${cfg.cluster}/tokens"

    mkdir -p "$cache"
    vault_file="$cache/vault.token"
    consul_file="$cache:/consul.token"
    nomad_file="$cache/nomad.token"

    # Vault

    if [ -s "$vault_file" ]; then
      VAULT_TOKEN="$(< "$vault_file")"
      export VAULT_TOKEN
    fi

    if ! vault token lookup &> /dev/null; then
      echo "Obtaining and caching Vault token"
      VAULT_TOKEN="$(
        github_token | \
          vault login -no-store -token-only -method=github -path github-employees token=-
      )"
      export VAULT_TOKEN
    fi

    vault print token \
      > "$vault_file"

    if is_admin; then
      ROLE="admin"
    else
      ROLE="developer"
    fi

    # Nomad

    if [ -s "$nomad_file" ]; then
      NOMAD_TOKEN="$(< "$nomad_file")"
      export NOMAD_TOKEN
    fi

    if ! nomad acl token self &> /dev/null; then
      echo "Obtaining and caching Nomad token for $ROLE"
      NOMAD_TOKEN="$(vault read -field secret_id nomad/creds/${ROLE})"
      export NOMAD_TOKEN
    fi

    nomad acl token self \
      | awk '/Secret ID/ { print $4 }' \
      > "$nomad_file"

    # Consul

    if [ -s "$consul_file" ]; then
      CONSUL_HTTP_TOKEN="$(< "$consul_file")"
      export CONSUL_HTTP_TOKEN
    fi

    if ! consul acl token read -self &> /dev/null; then
      echo "Obtaining and caching Consul token for $ROLE"
      CONSUL_HTTP_TOKEN="$(vault read -field token consul/creds/${ROLE})"
      export CONSUL_HTTP_TOKEN
    fi

    consul acl token read -self -format json \
      | jq -r -e .SecretID \
      > "$consul_file"
  '';

  awsOptions = {
    region = mkOption {
      description = "The amazon region";
      type = types.str;
    };
    asgRegions = mkOption {
      description = "The amazon auto-scaling regions; ':'-separated";
      type = types.str;
    };
    profile = mkOption {
      description = "The amazon profile";
      default = "";
      type = types.str;
    };
  };

in
{
  options.bitte.cli = {
    enable = mkEnableOption "setup the environment for bitte-cli";

    provider = mkOption {
      description = "The hosting provider environment";
      default = {};
      type = types.submodule {
        aws = awsOptions;
      };
    };

    defaults = {
      cluster = mkOption {
        description = "The default nomad cluster to work on";
        type = types.str;
      };
      domain = mkOption {
        description = "The default nomad cluster's domain";
        type = types.str;
      };
      namespace = mkOption {
        description = "The default nomad namespace within the default cluster";
        default = "";
        type = types.str;
      };
    };
  };

  config.devshell = optionalAttrs cfg.enable {

    imports = [ ./nix-devshell-temp-fix.nix ];

    assertions = [
      {
        assertion = cfg.provider != {};
        message = "at least one provider must be configured";
      }
      {
        assertion = (builtins.length (builtins.attrNames cfg.provider)) > 1;
        message = "at most one provider can be configured";
      }
    ];

    packages = with pkgs; [
      awscli
      # cfssl
      consul-template
      jq
      load-credentials
      nixos-rebuild
      openssl
      # python38Packages.pyhcl
      # scaler-guard
      terraform-with-plugins
    ];
    commands = [
      { category = "bitte"; package = pkgs.bitte; }
      { category = "bitte"; package = pkgs.consul; }
      { category = "bitte"; package = pkgs.nomad; }
      { category = "bitte"; package = pkgs.vault-bin; }
      { category = "bitte"; package = pkgs.sops; }
      { category = "nix"; package = pkgs.nixfmt; }
    ];

    env = [
      { name = "BITTE_CLUSTER"; value = cfg.cluster; }
      { name = "BITTE_DOMAIN"; value = cfg.domain; }
      { name = "BITTE_PROVIDER"; value = if (cfg.provider ? aws) then "AWS" else ""; }
      { name = "NOMAD_NAMESPACE"; value = cfg.namespace; }
      { name = "VAULT_ADDR"; value = "https://vault.${cfg.domain}"; }
      { name = "NOMAD_ADDR"; value = "https://nomad.${cfg.domain}"; }
      { name = "CONSUL_HTTP_ADDR"; value = "https://consul.${cfg.domain}"; }
    ]
    ++
    lib.optionals (cfg.provider ? aws) [
      { name = "AWS_PROFILE"; value = cfg.provider.aws.profile; }
      { name = "AWS_DEFAULT_REGION"; value = cfg.provider.aws.region; }
      { name = "AWS_ASG_REGIONS"; value = cfg.provider.asgRegions; }
    ];

    startup.load-credentials.text = "
      $DEVSHELL_DIR/bin/load-credentials
    ";
  };
}

