{ naersk
, pkg-config
, openssl
, zlib }:

naersk.buildPackage {
  # Without this we end up with a drv called `rust-workspace-unknown`
  # which makes `nix run` try to execute a bin with that name.
  inherit (with builtins; (fromTOML (readFile ./cli/Cargo.toml)).package)
  name version;
  root = ./.;
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ openssl zlib ];

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
