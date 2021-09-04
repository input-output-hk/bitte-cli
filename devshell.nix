{ config, lib, pkgs, ... }:
{

  name = "bitte-cli";
  env = [
    { name = "RUST_BACKTRACE"; value = "1"; }
    { name = "RUST_SRC_PATH"; value = "${pkgs.rustPlatform.rustLibSrc}"; }
  ];

  packages =  with pkgs; [
    cfssl
    sops
    openssl
    zlib
    pkg-config
    rust-analyzer
    cargo
    clippy
    rls
    rustc
    rustfmt
  ]
  ++
  lib.optionals stdenv.isDarwin (with darwin;
    with apple_sdk.frameworks; [
      libiconv
      libresolv
      Libsystem
      SystemConfiguration
      Security
      CoreFoundation
    ]
  );
}
