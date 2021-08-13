pub mod certs;
pub mod consul;
pub mod error;
pub mod info;
pub mod nomad;
pub mod rebuild;
pub mod ssh;
pub mod terraform;
pub mod types;

use error::Error;
pub(crate) type Result<T> = std::result::Result<T, Error>;

use anyhow::Context;
use execute::Execute;
use log::debug;
use std::env;
use std::process::Command;
use std::process::Stdio;

pub fn get_env(name: &str) -> anyhow::Result<String> {
    let value = env::var(name);
    value.with_context(|| format!("{} is not set", name))
}

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
                    })
                }
            }
            None => Err(Error::ExeError {
                details: "interrupted".to_string(),
            }),
        },
        Err(e) => Err(Error::ExeError {
            details: e.to_string(),
        }),
    }
}

fn handle_command_error(command: std::process::Command) -> Result<String> {
    handle_command_error_common(command, false)
}

pub fn sh(command: std::process::Command) -> Result<String> {
    handle_command_error_common(command, true)
}

fn check_cmd(cmd: &mut Command) -> Result<()> {
    println!("run: {:?}", cmd);
    cmd.status()?;

    Ok(())
}

#[derive(Clone)]
pub struct Instance {
    pub public_ip: String,
    pub name: String,
    pub uid: String,
    pub flake_attr: String,
    pub s3_cache: String,
}

impl Instance {
    pub fn new(
        public_ip: String,
        name: String,
        uid: String,
        flake_attr: String,
        s3_cache: String,
    ) -> Instance {
        Instance {
            public_ip,
            name,
            uid,
            flake_attr,
            s3_cache,
        }
    }
}
