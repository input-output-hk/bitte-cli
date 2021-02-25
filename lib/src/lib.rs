pub mod certs;
pub mod info;
pub mod rebuild;
pub mod ssh;
pub mod terraform;
pub mod types;

use execute::Execute;
use std::process::Command;
use std::env;
use std::{fmt, process::Stdio};
use anyhow::{Result, Context};

use info::{asg_info, instance_info};

pub fn bitte_cluster() -> Result<String> {
    Ok(env::var("BITTE_CLUSTER")
    .context("BITTE_CLUSTER environment variable must be set")?)
}

fn handle_command_error(mut command: std::process::Command) -> Result<String, ExeError> {
    println!("run: {:?}", command);
    // command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    match command.execute_output() {
        Ok(output) => match output.status.code() {
            Some(exit_code) => {
                if exit_code == 0 {
                    Ok("Ok".to_string())
                } else {
                    Err(ExeError {
                        details: String::from_utf8_lossy(&output.stderr).to_string(),
                    })
                }
            }
            None => Err(ExeError {
                details: "interrupted".to_string(),
            }),
        },
        Err(e) => Err(ExeError {
            details: e.to_string(),
        }),
    }
}

#[derive(Debug)]
struct ExeError {
    details: String,
}

impl fmt::Display for ExeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl std::error::Error for ExeError {
    fn description(&self) -> &str {
        &self.details
    }
}

fn check_cmd(cmd: &mut Command) {
    println!("run: {:?}", cmd);
    cmd.status()
        .expect(format!("failed to run: {:?}", cmd).as_str());
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

pub async fn find_instance(needle: &str) -> Option<Instance> {
    match find_instances(vec![needle]).await.first() {
        Some(instance) => Some(instance.clone()),
        None => None,
    }
}

async fn find_instances(patterns: Vec<&str>) -> Vec<Instance> {
    let current_state_version = terraform::fetch_current_state_version("clients")
        .or_else(|_| terraform::fetch_current_state_version("core"))
        .expect("Coudln't fetch clients or core workspaces");

    let output = terraform::current_state_version_output(&current_state_version)
        .expect("Problem loading state version from terraform");

    let mut results = Vec::new();

    for instance in output.instances.values().into_iter() {
        if patterns.iter().any(|pattern| {
            [
                instance.private_ip.as_str(),
                instance.public_ip.as_str(),
                instance.name.as_str(),
            ]
            .contains(pattern)
        }) {
            results.push(Instance::new(
                instance.public_ip.to_string(),
                instance.name.to_string(),
                instance.uid.to_string(),
                instance.flake_attr.to_string(),
                output.s3_cache.to_string(),
            ));
        }
    }

    if let Some(asgs) = output.asgs {
        for (_, asg) in asgs {
            let asg_infos = asg_info(asg.arn.as_str(), asg.region.as_str()).await;
            for asg_info in asg_infos {
                let instance_infos =
                    instance_info(asg_info.instance_id.as_str(), asg.region.as_str()).await;
                for instance_info in instance_infos {
                    if patterns.iter().any(|pattern| {
                        [
                            instance_info.instance_id.as_ref(),
                            instance_info.public_dns_name.as_ref(),
                            instance_info.public_ip_address.as_ref(),
                            instance_info.private_dns_name.as_ref(),
                            instance_info.private_ip_address.as_ref(),
                        ]
                        .iter()
                        .map(|x| x.map_or_else(|| "", |y| y.as_str()))
                        .collect::<String>()
                        .contains(pattern)
                    }) {
                        if let Some(ip) = instance_info.public_ip_address {
                            results.push(Instance::new(
                                ip,
                                instance_info
                                    .instance_id
                                    .map_or_else(|| "".to_string(), |x| x),
                                asg.uid.clone(),
                                asg.flake_attr.clone(),
                                output.s3_cache.clone(),
                            ))
                        }
                    }
                }
            }
        }
    }

    results
}
