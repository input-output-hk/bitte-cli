mod common;

use anyhow::Result;
use assert_cmd::prelude::*;
use common::*;
use predicate::str::*;
use predicates::prelude::*;
use assert_fs::prelude::*;
use std::process::Command;
use rstest::{fixture,rstest};
use tempfile::tempdir;

// Using mod names for context saves some typing compared to fn name prefixes.
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
                                               //     test!(BITTE_CLUSTER="lies";
                                               //           bitte tf network plan;
                                               //           should fail with "TERRAFORM_ORGANIZATION environment variable must be set");
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
                                               //     test!(BITTE_CLUSTER="lies" TERRAFORM_ORGANIZATION="moar_lies";
                                               //           bitte tf network plan;
                                               //           should fail with "is not a flake");
}

#[test]
#[ignore]
// Leaving this here as reference
fn test_tempdir() {
    let tempdir = tempfile::tempdir().expect("Could not create tempdir");
    let mut cmd = Command::new("pwd");

    // tempdir *MUST* be passed as reference because otherwise the ownership transfers to
    // `current_dir()` which results in the destructor running right after it and the tempdir
    // erased before we do anything. This yields a misleading "No such file or directory"
    // error which appears to be relating to the binary we're trying to run rather than the
    // directory we're trying to run it in. See:
    // https://github.com/Stebalien/tempfile/issues/115
    cmd.current_dir(&tempdir)
        .assert()
        .success()
        .stdout(contains(tempdir.path().to_str().unwrap()));
    ()
}

#[test]
#[ignore]
// Haven't figured out yet how to get assert_fs's tempdir with assert_cmd. this can be nice as it
// uses the same syntax as assert_cmd.
fn test_with_assertfs() -> Result<()> {
    let temp = assert_fs::TempDir::new().unwrap();
    let input_file = temp.child("foo.txt");
    input_file.touch().unwrap();
    // ... do something with input_file ...
    input_file.assert("");
    temp.child("bar.txt").assert(predicate::path::missing());
    temp.close().unwrap();
    Ok(())
}

// rstest crate lets us annotate functions as fixtures
#[fixture]
fn bitte() -> Command {
    Command::cargo_bin("bitte").unwrap()
}

// and then receive their values in our test functions annotated with #[rstest]. kinda like nix's
// callPackages
#[rstest]
fn test_fixtures(mut bitte: Command) {
    bitte
        .current_dir(&(tempdir().unwrap()))
        .assert()
        .failure()
        .stderr(contains("Unknown command"));
    ()
}