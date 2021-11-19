{ stdenv
, lib
, pkg-config
, openssl
, zlib
, makeRustPlatform
, fenix
  # darwin dependencies
, darwin
, toolchain
, self
}:

(makeRustPlatform { inherit (fenix.${toolchain}) cargo rustc; }).buildRustPackage
  {

    inherit (with builtins; (fromTOML (readFile "${self}/Cargo.toml")).package)
      name version;

    src = self;
    cargoLock.lockFile = "${self}/Cargo.lock";
    cargoLock.outputHashes = {
      "deploy-rs-0.1.0" = "sha256-g9c52aWBbwC/J3mdfUNp9EoWSI2xNyYts51jR68RKIU=";
    };

    nativeBuildInputs = [ pkg-config ];
    buildInputs = [ openssl zlib ] ++ lib.optionals stdenv.isDarwin
      (with darwin.apple_sdk.frameworks; [
        SystemConfiguration
        Security
        CoreFoundation
        darwin.libiconv
        darwin.libresolv
        darwin.Libsystem
      ]);

    doCheck = false;

    postInstall = ''
      export BITTE_CLUSTER=b
      export BITTE_PROVIDER=aws
      export BITTE_DOMAIN=b.b.b

      mkdir -p $out/share/zsh/site-functions
      $out/bin/bitte comp zsh > $out/share/zsh/site-functions/_bitte

      mkdir -p $out/share/bash-completion/completions
      $out/bin/bitte comp bash > $out/share/bash-completion/completions/bitte
    '';
  } // {
  meta.description = "A swiss knife for the bitte cluster";
}
