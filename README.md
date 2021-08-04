# Bitte CLI

This is a little tool that helps with deployments of Bitte clusters.

Bitte is a set of NixOS configurations that are provisioned using Terraform and
runs a cluster of Consul, Vault, and Nomad instances.

## Build this using nix

    nix build -o bitte

## Run this

    ./bitte --help

### Install cli tools outside of nix

To install the bitte tools, you will also need the following dependencies:
- [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
- pkg-config
- openssl (linux only, darwin will use Security framework)
- zlib

To install:
```bash
  cargo install --path cli  # for the bitte cli
  cargo install --path iogo # for the iogo utility
```

## Setup the Bitte Environment

    export BITTE_FLAKE=git+ssh://git@github.com/input-output-hk/bitte
    export BITTE_CLUSTER=cvn-testnet
    export AWS_DEFAULT_REGION=eu-central-1
    export AWS_PROFILE=cvn-testnet

## Detailed Workflow

    bitte terraform
    terraform init
    terraform plan -out cvn-testnet.plan
    terraform apply cvn-testnet.plan
    bitte certs
    bitte deploy
