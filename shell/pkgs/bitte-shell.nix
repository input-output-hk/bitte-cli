{ devshell, bitteDevshellModule, lib }:

{ self
, cluster ? builtins.head (builtins.attrNames self.clusters)
, caCert ? null
, domain ? self.clusters.${cluster}._proto.config.cluster.domain
, extraPackages ? [ ]
, region ? self.clusters.${cluster}._proto.config.cluster.region or ""
, profile ? ""
, provider ? "AWS"
, namespace ? cluster
, awsAutoScalingGroups ? self.clusters.${cluster}._proto.config.cluster.awsAutoScalingGroups
}: lib.warn ''

Use of pkgs.bitteShell is deprecated, please update to direct usage of bitte.inputs.cli.devshellModules.bitte

add in your toplevel flake (in appropriate places):
- inputs.devshell.url = "github:numtide/devshell";
- outputs = { ..., devshell, ... }: ...
- overlays = [ ... devshell.overlay ... ];
- devShell = pkgs.devshell.mkShell {
  imports = [ bitte.inputs.cli.devshellModules.bitte ];
  commands =
    let
      withCategory = category: attrset: attrset // { inherit category; };
      custom = withCategory "custom";
    in [ # TODO: fix package name as needed
      ${lib.concatStringsSep "\n      " (map (p: "(custom { package = pkgs.${(builtins.parseDrvName p.name).name}; })") extraPackages)}
    ];
  bitte = {
    cluster = "${cluster}";
    domain = "${domain}";
    namespace = "${namespace}";
    provider = "${provider}";
    cert = ${if caCert == null then "null" else caCert};
    aws_region = "${region}";
    aws_profile = "${profile}";
    aws_autoscaling_groups = self.clusters.${cluster}._proto.config.cluster.awsAutoScalingGroups;
  };
};

Example implementation: https://github.com/input-output-hk/erc20-ops/commit/af390f4831033522d14eb87bc7618be1cd7b569f

Yours sincerely,

Ihre Heinzelmännchen
''
devshell.mkShell {
  imports = [ bitteDevshellModule ];
  packages = extraPackages;
  bitte = { inherit cluster domain namespace provider; };
  bitte.cert = caCert;
  bitte.aws_region = region;
  bitte.aws_profile = profile;
  bitte.aws_autoscaling_groups = awsAutoScalingGroups;
}
