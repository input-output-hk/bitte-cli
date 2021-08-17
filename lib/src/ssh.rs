use std::net::IpAddr;
use std::time::Duration;
use std::{path::Path, process::Command};
use tokio::{net::TcpStream, time};

use super::check_cmd;
use crate::error::Error;
use anyhow::Result;

pub fn ssh_keygen(ip: &IpAddr) -> Result<()> {
    check_cmd(Command::new("ssh-keygen").arg("-R").arg(ip.to_string()))
        .map_err(|_| Error::Unknown)?;
    Ok(())
}

pub fn wait_for_ready(cluster: &str, ip: &IpAddr) -> Result<()> {
    let target = format!("root@{}", ip);

    let mut ssh_args = vec![
        "-C", // Requests compression of all data
        "-o",
        "NumberOfPasswordPrompts=0",
        "-o",
        "ServerAliveInterval=60",
        "-o",
        "ControlPersist=600",
        "-o",
        "StrictHostKeyChecking=accept-new",
    ];

    let ssh_key_path = format!("secrets/ssh-{}", cluster);
    let ssh_key = Path::new(&ssh_key_path);
    if ssh_key.is_file() {
        ssh_args.push("-i");
        ssh_args.push(ssh_key_path.as_str());
    }

    ssh_args.push(&target);
    ssh_args.push("until grep true /etc/ready &>/dev/null; do sleep 1; done");
    check_cmd(Command::new("ssh").args(ssh_args)).map_err(|_| Error::Unknown)?;
    Ok(())
}

pub async fn wait_for_ssh(ip: &IpAddr) -> Result<()> {
    let res = wait_for_port(&ip, 22, 10000, 120).await?;
    Ok(res)
}

pub async fn wait_for_port(
    ip: &IpAddr,
    port: usize,
    duration_in_ms: u64,
    attempts: usize,
) -> Result<()> {
    let addr = format!("{}:{}", ip, port);
    let timeout_duration = Duration::from_millis(duration_in_ms);
    let mut interval = time::interval(timeout_duration);

    for _i in 0..attempts {
        let stream = TcpStream::connect(addr.clone());
        let timeout = time::timeout(timeout_duration, stream);
        match timeout.await {
            Ok(o) => match o {
                Ok(_) => {
                    return Ok(());
                }
                Err(ee) => {
                    println!("error while connecting: {}", ee);
                    interval.tick().await;
                }
            },
            Err(e) => println!("Waiting for {} to respond: {}", addr, e),
        }
    }
    Err(Error::ExhaustedAttempts(attempts).into())
}
