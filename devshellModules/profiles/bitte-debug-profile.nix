{
  imports = [ ../bitte-cli.nix ];

  env = [
    { name = "RUST_LOG"; value = "debug"; }
  ];

}
