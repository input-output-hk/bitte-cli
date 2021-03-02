mod common;

use anyhow::Result;
use assert_cmd::prelude::*;
use common::*;
use predicate::str::*;
use predicates::prelude::*; // Used for writing assertions
use std::process::Command;

// I'm trying to abstract away as much boilerplate as possible.
// For reference, a "normal" test looks like this:
#[test]
fn test_bitte_tf_network_plan_with_both_terraform_organization_and_bitte_cluster_set() -> Result<()> {
    // ☝️ nobody wants to write this!
    Command::cargo_bin("bitte")?
        .args(&["tf", "network", "plan"])
        .env("TERRAFORM_ORGANIZATION", "lies")
        .env("BITTE_CLUSTER", "lies")
        .assert()
        .failure()
        .stderr(contains("is not a flake"));

    Ok(())
}

// Using mod names for context saves some typing compared to fn name prefices.
mod with_no_env {
    use super::test;
    // the test! macro is defined in common/mod.rs
    // it currently supports only failures 🙃
    test!(bitte; should fail with "Unknown command");
    test!(bitte tf; should fail with "bitte terraform <workspace>");
    test!(bitte tf network; should fail with "Unknown command"); // <- not really but it does.
    test!(bitte tf network plan; should fail with "BITTE_CLUSTER");
}

mod with_bitte_cluster {
    use super::test;
    // We can give test! environment variables as well.
    // I'd like to be able to scope that to the mod somehow.
    test!(BITTE_CLUSTER="lies";
          bitte;
          should fail with "Unknown command");
    test!(BITTE_CLUSTER="lies";
          bitte tf;
          should fail with "bitte terraform <workspace>");
    test!(BITTE_CLUSTER="lies";
          bitte tf network;
          should fail with "Unknown command"); // <- not really but it does.
    test!(BITTE_CLUSTER="lies";
          bitte tf network plan;
          should fail with "TERRAFORM_ORGANIZATION environment variable must be set");
}

mod with_terraform_organization {
    use super::test;
    test!(TERRAFORM_ORGANIZATION="moar_lies";
          bitte;
          should fail with "Unknown command");
    test!(TERRAFORM_ORGANIZATION="moar_lies";
          bitte tf;
          should fail with "bitte terraform <workspace>");
    test!(TERRAFORM_ORGANIZATION="moar_lies";
          bitte tf network;
          should fail with "Unknown command"); // <- not really but it does.
    test!(TERRAFORM_ORGANIZATION="moar_lies";
          bitte tf network plan;
          should fail with "BITTE_CLUSTER environment variable must be set");
}

mod with_terraform_organization_and_bitte_cluster {
    use super::test;
    test!(BITTE_CLUSTER="lies" TERRAFORM_ORGANIZATION="moar_lies";
          bitte;
          should fail with "Unknown command");
    test!(BITTE_CLUSTER="lies" TERRAFORM_ORGANIZATION="moar_lies";
          bitte tf;
          should fail with "bitte terraform <workspace>");
    test!(BITTE_CLUSTER="lies" TERRAFORM_ORGANIZATION="moar_lies";
          bitte tf network;
          should fail with "Unknown command"); // <- not really but it does.
    test!(BITTE_CLUSTER="lies" TERRAFORM_ORGANIZATION="moar_lies";
          bitte tf network plan;
          should fail with "is not a flake");
}