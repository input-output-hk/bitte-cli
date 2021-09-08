{ stdenv, lib, pkg-config, openssl, zlib, rustPlatform, naersk
# darwin dependencies
, darwin }:

naersk.buildPackage {
  # Without this we end up with a drv called `rust-workspace-unknown`
  # which makes `nix run` try to execute a bin with that name.
  inherit (with builtins; (fromTOML (readFile ./cli/Cargo.toml)).package)
    name version;
  root = ./.;
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
}
