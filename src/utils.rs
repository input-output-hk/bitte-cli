pub mod certs;
pub mod error;
pub mod nomad;
pub mod terraform;
pub mod types;

use error::Error;

use anyhow::{anyhow, Result};
use std::process::Command;

fn check_cmd(cmd: &mut Command) -> Result<()> {
    println!("run: {:?}", cmd);
    let status = cmd.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!(
            "{:?} failed with non-zero exit code {:?}",
            cmd,
            status
        ))
    }
}

#[derive(Clone)]
pub struct Instance {
    pub public_ip: String,
    pub name: String,
    pub uid: String,
    pub flake_attr: String,
    pub s3_cache: String,
}
