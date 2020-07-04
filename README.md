# Bitte CLI

This is a little tool that helps with deployments of Bitte clusters.

Bitte is a set of NixOS configurations that are provisioned using Terraform and
runs a cluster of Consul, Vault, and Nomad instances.

## Build this

    nix build -o bitte

## Run this

    ./bitte --help

## Setup the Environment

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
