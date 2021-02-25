mod common;
use common::*;

use anyhow::Result;
use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions

// bitte!();

#[test]
fn bitte_no_args_does_fail_stderr() -> Result<()> {
    bitte()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown command"));

    Ok(())
}

#[test]
fn bitte_tf_network_plan_given_noenv() -> Result<()> {
    // cmd.env("BITTE_CLUSTER", "lies");
    bitte()
        .args(&["tf", "network", "plan"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("BITTE_CLUSTER"));

    Ok(())
}

#[test]
fn bitte_tf_network_plan_given_just_bitte_cluster() -> Result<()> {
    bitte()
        .env("BITTE_CLUSTER", "lies")
        .args(&["tf", "network", "plan"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("TERRAFORM_ORGANIZATION"));

    Ok(())
}

#[test]
fn bitte_tf_network_plan_given_just_terraform_organization() -> Result<()> {
    bitte()
        .env("TERRAFORM_ORGANIZATION", "lies")
        .args(&["tf", "network", "plan"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("BITTE_CLUSTER"));

    Ok(())
}

#[test]
fn bitte_tf_network_plan_given_both_bitte_cluster_and_terraform_organization() -> Result<()> {
    bitte()
        .env("TERRAFORM_ORGANIZATION", "lies")
        .env("BITTE_CLUSTER", "lies")
        .args(&["tf", "network", "plan"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("is not a flake"));

    Ok(())
}
