{ stdenv, makeWrapper, crystal, inclusive, nixFlakes, nixos-rebuild, openssh
, awscli, gitMinimal, coreutils, gnugrep, terraform-with-plugins
, consul, sops, libssh2, pkgconfig, cfssl, rsync, openssl, vault-bin }: {
  bitte = let
    inner = crystal.buildCrystalPackage {
      pname = "bitte-cli";
      version = "0.1.0";
      format = "crystal";

      src = inclusive ./. [ ./shard.lock ./shard.yml ./src ];

      buildInputs = [ libssh2 openssl ];

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
      terraform-with-plugins
      cfssl
      rsync
      vault-bin
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
