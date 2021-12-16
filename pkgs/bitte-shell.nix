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
}: devshell.mkShell {
  imports = [ bitteDevshellModule ];
  packages = extraPackages;
  bitte = { inherit cluster domain namespace provider; };
  bitte.cert = caCert;
  bitte.aws_region = region;
  bitte.aws_profile = profile;
  bitte.aws_autoscaling_groups = awsAutoScalingGroups;
}
