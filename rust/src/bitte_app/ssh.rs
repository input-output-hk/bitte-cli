use std::{path::Path, process::Command};

use clap::ArgMatches;

use super::{bitte_cluster, find_instance};

pub async fn cli_ssh(sub: &ArgMatches) {
    let needle: String = sub.value_of_t_or_exit("host");
    let mut args = sub.values_of_lossy("args").unwrap_or(vec![]);

    let ip = find_instance(needle.as_str())
        .await
        .map_or_else(|| needle.clone(), |i| i.public_ip.clone());
    let user_host = format!("root@{}", ip);
    let mut flags = vec!["-x".to_string(), "-p".into(), "22".into()];

    let ssh_key_path = format!("secrets/ssh-{}", bitte_cluster());
    let ssh_key = Path::new(&ssh_key_path);
    if ssh_key.is_file() {
        flags.push("-i".to_string());
        flags.push(ssh_key_path.to_string());
    }

    flags.push(user_host.to_string());
    flags.append(args.as_mut());

    if args.len() > 0 {
        flags.append(&mut vec!["-t".to_string()]);
    }
    let ssh_args = flags.into_iter();

    let mut cmd = Command::new("ssh");
    let cmd_with_args = cmd.args(ssh_args);
    println!("cmd: {:?}", cmd_with_args);

    cmd.spawn()
        .expect("ssh command failed")
        .wait()
        .expect("ssh command didn't finish?");
}
