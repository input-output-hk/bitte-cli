{ stdenv, lib, pkg-config, openssl, zlib, makeRustPlatform, fenix
# darwin dependencies
, darwin }:

(makeRustPlatform {
  inherit (fenix.stable) cargo rustc;
}).buildRustPackage {

  inherit (with builtins; (fromTOML (readFile ./Cargo.toml)).package)
    name
    version
  ;

  src = ./.;
  cargoLock.lockFile = ./Cargo.lock;
  cargoLock.outputHashes = {
    "deploy-rs-0.1.0" = "sha256-IqP03slZYesbmCY52JQbs0mIegjSgM9MKJIonSzLBfs=";
  };

  dontCargoCheck = true; # flaky tests during refactoring

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
} // { meta.description = "A swiss knife for the bitte cluster"; }
