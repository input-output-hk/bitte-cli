# `numtide/devshell`

https://numtide.github.io/devshell/

This folder is an inventory of authoritative devshell modules & profiles
that shall be used throughout downstream adopters of this repo.

The shared config namespace for all bitte related devshells is to be:

```nix
{
  config.bitte
  # config.bitte.cli
  # config.bitte.ci
}
```

## Modules
Abstract declarations

## Profiles
Concrete definitons

## Usage

```nix
{
  inputs.bitte = "github:....";
  outputs = { bitte }: {
    bitte.mkFlake {
      # ...
      devshell.externalModules = [
        bitte.devshellModules.bitte-cli
        bitte.devshellModules.bitte-ci
      ];
      devshell.modules = [ ./devshell.toml ];
    };
  };
}
```
