use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions
use std::process::Command; // Run programs

#[test]
fn bitte() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("bitte")?;

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Unknown command"));

    Ok(())
}

#[test]
fn bitte_tf_network_plan() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("bitte")?;

    cmd.arg("tf")
        .arg("network")
        .arg("plan");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Unknown command"));

    Ok(())
}
