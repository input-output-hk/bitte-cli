use std::{path::Path, process::Command};
use tokio::{net::TcpStream, time::timeout};
use std::time::Duration;

use super::check_cmd;

pub fn ssh_keygen(ip: &String) {
    check_cmd(Command::new("ssh-keygen").arg("-R").arg(ip));
}

pub fn wait_for_ready(cluster: &String, ip: &String) {
    let target = format!("root@#{}", ip);

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
    check_cmd(Command::new("ssh").args(ssh_args))
}

pub async fn wait_for_ssh(ip: &String) {
    let addr = format!("{}:22", ip);

    for i in 0..120 {
        let stream = TcpStream::connect(addr.clone());
        let t = timeout(Duration::from_millis(10000), stream);
        match t.await {
            Ok(o) => match o {
                Ok(_) => {
                    return;
                }
                Err(ee) => {
                    if i >= 120 {
                        println!("error while connecting: {}", ee);
                    }
                }
            },
            Err(e) => println!("Waiting for {} to respond: {}", addr, e),
        }
    }
}
