{ stdenv
, makeWrapper
, crystal
, nixos-rebuild
, openssh
, awscli
, gitMinimal
, coreutils
, gnugrep
, sops
, libssh2
, pkgconfig
, cfssl
, rsync
, openssl
}:
let
  inner = crystal.buildCrystalPackage {
    pname = "bitte";
    version = "0.1.0";
    format = "crystal";

    src = ./.;

    buildInputs = [ libssh2 openssl ];

    shardsFile = ./shard.nix;

    crystalBinaries.bitte = {
      src = "src/bitte.cr";
      options = [ "--verbose" "--debug" "-Dflag" ];
    };
  };

  PATH = stdenv.lib.makeBinPath [
    awscli
    coreutils
    gitMinimal
    gnugrep
    nixos-rebuild
    openssh
    sops
    cfssl
    rsync
  ];

in
stdenv.mkDerivation {
  pname = inner.pname;
  version = inner.version;

  nativeBuildInputs = [ makeWrapper ];
  src = inner;

  installPhase = ''
    mkdir -p $out/bin
    cp $src/bin/bitte $out/bin/bitte
    wrapProgram $out/bin/bitte \
      --prefix PATH ":" ${PATH}
  '';

  passthru.exePath = "/bin/bitte";
}
