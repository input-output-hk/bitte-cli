pub mod certs;
pub mod error;
pub mod nomad;
pub mod rebuild;
pub mod ssh;
pub mod terraform;
pub mod types;

use error::Error;

use anyhow::{anyhow, Result};
use execute::Execute;
use log::debug;
use std::process::Command;
use std::process::Stdio;

fn handle_command_error_common(
    mut command: std::process::Command,
    pipe_stdout: bool,
) -> Result<String> {
    debug!("run: {:?}", command);
    if pipe_stdout {
        command.stdout(Stdio::piped());
    }
    command.stderr(Stdio::piped());

    match command.execute_output() {
        Ok(output) => match output.status.code() {
            Some(exit_code) => {
                if exit_code == 0 {
                    Ok(String::from_utf8_lossy(output.stdout.as_slice()).to_string())
                } else {
                    Err(Error::ExeError {
                        details: String::from_utf8_lossy(&output.stderr).to_string(),
                    }
                    .into())
                }
            }
            None => Err(Error::ExeError {
                details: "interrupted".to_string(),
            }
            .into()),
        },
        Err(e) => Err(Error::ExeError {
            details: e.to_string(),
        }
        .into()),
    }
}

fn handle_command_error(command: std::process::Command) -> Result<String> {
    handle_command_error_common(command, false)
}

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
