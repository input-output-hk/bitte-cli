{ stdenv, makeWrapper, crystal, inclusive, nixFlakes, nixos-rebuild, openssh
, awscli, gitMinimal, coreutils, systemd, gnugrep, terraform-with-plugins
, consul, sops, libssh2, pkgconfig, cfssl, rsync }: {
  bitte = let
    inner = crystal.buildCrystalPackage {
      pname = "bitte-cli";
      version = "0.1.0";
      format = "crystal";

      src = inclusive ./. [ ./shard.lock ./shard.yml ./src ];

      buildInputs = [ libssh2 ];

      shardsFile = ./shard.nix;

      crystalBinaries.bitte = {
        src = "src/bitte.cr";
        options = [ "--verbose" "--debug" ];
      };
    };

    PATH = stdenv.lib.makeBinPath [
      awscli
      consul
      coreutils
      gitMinimal
      gnugrep
      nixFlakes
      nixos-rebuild
      openssh
      sops
      systemd
      terraform-with-plugins
      cfssl
      rsync
    ];

  in stdenv.mkDerivation {
    pname = inner.pname;
    version = inner.version;

    nativeBuildInputs = [ makeWrapper ];
    src = inner;

    installPhase = ''
      mkdir -p $out/bin
      cp $src/bin/bitte $out/bin/bitte
      wrapProgram $out/bin/bitte \
        --set PATH ${PATH}
    '';
  };
}
