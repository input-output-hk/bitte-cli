mod common;

use anyhow::Result;
use assert_cmd::prelude::*;
use predicates::prelude::*; // Used for writing assertions
use predicate::str::*;
use std::process::Command;

mod macros {
    use paste::paste;

    macro_rules! amacro {
        ($($x:expr)*) => {
            &[$(stringify!{$x}),*]
        };
    }
    #[test]
    fn test_macro() {
        assert_eq!(amacro!(hello world), &["hello", "world"]);
        assert_eq!(amacro!(hello "world"), &["hello", "\"world\""])
    }

    macro_rules! anothermacro {
        ($($x:expr)*) => {
            paste! {
                stringify!{ [<$($x)_*>] }
            }
        };
    }

    #[test]
    fn test_anothermacro() {
        assert_eq!(anothermacro!(hello world), "hello_world");
        assert_eq!(anothermacro!(hello "world"), "hello_world")
    }
}

// macro_rules! bitte {
//     () => (Command::cargo_bin("bitte")?);
//     ($tf:expr) => { bitte!().arg($tf) };
//     ($tf:expr, $network:expr) => (bitte!($tf).arg($network));
//     ($tf:expr, $network:expr, $plan:expr) => (bitte!($tf).arg($network).arg($plan));
// }

macro_rules! bitte {
    ($($args:expr),*) => {
        Command::cargo_bin("bitte")?
        $(
            .arg($args)
        )*
    }
}
#[test]
fn bitte_no_args_does_fail_stderr() -> Result<()> {
    bitte!()
        .assert()
        .failure()
        .stderr(contains("Unknown command"));

    Ok(())
}

#[test]
fn bitte_tf() -> Result<()> {
    // cmd.env("BITTE_CLUSTER", "lies");
    bitte!("tf")
        .assert()
        .failure()
        .stderr(contains("bitte terraform <workspace>"));

    Ok(())
}

#[test]
fn bitte_tf_network() -> Result<()> {
    // cmd.env("BITTE_CLUSTER", "lies");
    // Command::cargo_bin("bitte")?
    //     .args(&["tf", "network"])
    bitte!("tf", "network")
        .assert()
        .failure()
        .stderr(contains("Unknown command"));

    Ok(())
}

#[test]
fn bitte_tf_network_plan_given_noenv() -> Result<()> {
    // cmd.env("BITTE_CLUSTER", "lies");
    // Command::cargo_bin("bitte")?
    //     .args(&["tf", "network", "plan"])
    bitte!("tf", "network", "plan")
        .assert()
        .failure()
        .stderr(contains("BITTE_CLUSTER"));

    Ok(())
}

#[test]
fn bitte_tf_network_plan_given_just_bitte_cluster() -> Result<()> {
    // Command::cargo_bin("bitte")?
    //     .args(&["tf", "network", "plan"])
    bitte!("tf", "network", "plan")
        .env("BITTE_CLUSTER", "lies")
        .assert()
        .failure()
        .stderr(contains("TERRAFORM_ORGANIZATION"));

    Ok(())
}

#[test]
fn bitte_tf_network_plan_given_just_terraform_organization() -> Result<()> {
    bitte!("tf", "network", "plan")
        .env("TERRAFORM_ORGANIZATION", "lies")
        .assert()
        .failure()
        .stderr(contains("BITTE_CLUSTER"));

    Ok(())
}

#[test]
fn bitte_tf_network_plan_given_both_bitte_cluster_and_terraform_organization() -> Result<()> {
    bitte!("tf", "network", "plan")
        .env("TERRAFORM_ORGANIZATION", "lies")
        .env("BITTE_CLUSTER", "lies")
        .assert()
        .failure()
        .stderr(contains("is not a flake"));

    Ok(())
}
