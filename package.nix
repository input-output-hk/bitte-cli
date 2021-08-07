{ stdenv
, lib
, naersk
, pkg-config
, openssl
, zlib

  # darwin dependencies
, darwin
}:

naersk.buildPackage {
  # Without this we end up with a drv called `rust-workspace-unknown`
  # which makes `nix run` try to execute a bin with that name.
  inherit (with builtins; (fromTOML (readFile ./cli/Cargo.toml)).package)
    name version;
  root = ./.;
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ openssl zlib ]
    ++ lib.optionals stdenv.isDarwin (with darwin.apple_sdk.frameworks; [
    SystemConfiguration
    Security
    CoreFoundation
    darwin.libiconv
    darwin.libresolv
    darwin.Libsystem
  ]);

  overrideMain = _: {
    postInstall = ''
            mkdir -p "$out/share/"{bash-completion/completions,fish/vendor_completions.d,zsh/site-functions}

            echo "generate completion scripts for iogo"
            $out/bin/iogo completions bash > "$out/share/bash-completion/completions/iogo"
            $out/bin/iogo completions fish > "$out/share/fish/vendor_completions.d/iogo.fish"
            $out/bin/iogo completions zsh >  "$out/share/zsh/site-functions/_iogo"
    '';
  };
}
