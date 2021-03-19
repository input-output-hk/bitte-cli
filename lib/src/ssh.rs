use anyhow::{Result, bail};
use std::{path::Path, process::Command};
use tokio::{net::TcpStream, time};
use std::time::Duration;

use super::check_cmd;

pub fn ssh_keygen(ip: &str) -> Result<()> {
    check_cmd(Command::new("ssh-keygen").arg("-R").arg(ip))?;
    Ok(())
}

pub fn wait_for_ready(cluster: &str, ip: &str) -> Result<()> {
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
    check_cmd(Command::new("ssh").args(ssh_args))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::ssh::wait;

    #[tokio::test]
    async fn test_wait() {
        let result = wait("169.254.1.2", 1).await;
        assert!(result.is_err());
    }
}

pub async fn wait_for_ssh(ip: &str) -> Result<()> {
    let res = wait(&ip, 120).await?;
    Ok(res)
}

pub async fn wait(ip: &str, timeout: u8) -> Result<()> {
    let addr = format!("{}:22", ip);

    for i in 0..timeout {
        let stream = TcpStream::connect(addr.clone());
        let t = time::timeout(Duration::from_millis(1000), stream);
        match t.await {
            Ok(o) => match o {
                Ok(_) => {
                    return Ok(());
                }
                Err(ee) => {
                    if i >= timeout {
                        println!("error while connecting: {}", ee);
                    }
                }
            },
            Err(e) => println!("Waiting for {} to respond: {}", addr, e),
        }
    }
    bail!("Timeout waiting for {}", addr)
}
