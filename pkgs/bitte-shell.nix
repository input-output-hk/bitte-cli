{ devshell, bitteDevshellModule }:

{ self
, cluster ? builtins.head (builtins.attrNames self.clusters)
, caCert ? null
, domain ? self.clusters.${cluster}.proto.config.cluster.domain
, extraPackages ? [ ]
, region ? self.clusters.${cluster}.proto.config.cluster.region or ""
, profile ? ""
, provider ? "AWS"
, namespace ? cluster
, nixConfig ? null
, asg ? self.clusters.${cluster}.proto.config.cluster.autoscalingGroups
}:

in {
  inherit devshellModule;
  devshell = devshell.mkShell {
    imports = [ bitteDevshellModule ];
    packages = extraPackages;
    bitte = { inherit cluster domain namespace provider; };
    bitte.cert = caCert;
    bitte.aws_region = region;
    bitte.aws_profile = profile;
    bitte.aws_autoscaling_groups = asg;
    env = [{
      name = "NIX_CONFIG";
      value = ''
        extra-experimental-features = nix-command flakes ca-references recursive-nix
        allow-import-from-derivation = true
        substituters = https://cache.nixos.org https://hydra.iohk.io
        trusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY= hydra.iohk.io:f/Ea+s+dFdN+3Y/G+FDgSq+a5NEWhJGzdjvKNGv0/EQ=
      '' + (lib.optionalString (nixConfig != null) nixConfig);
    }];
  };
}
