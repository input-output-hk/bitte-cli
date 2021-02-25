use std::{env, path::Path, process::Command, time::Duration};
use log::info;
use anyhow::Result;

use super::{
    bitte_cluster, check_cmd, find_instances, handle_command_error, Instance,
    ssh::wait_for_ssh
};

pub async fn copy(only: Vec<&str>, delay: Duration) {
    let instances = find_instances(only.clone()).await;
    let mut iter = instances.iter().peekable();

    while let Some(instance) = iter.next() {
        info!("rebuild: {}", instance.name);
        wait_for_ssh(&instance.public_ip).await;
        copy_to(instance, 10);
        if iter.peek().is_some() {
            tokio::time::sleep(delay).await;
        }
    }
}

fn copy_to(instance: &Instance, _attempts: u64) {
    env::set_var("IP", instance.public_ip.clone());
    let flake = ".";

    handle_command_error(execute::command_args!(
        "nix",
        "run",
        format!(
            "{}#nixosConfigurations.{}.config.secrets.generateScript",
            flake, instance.uid
        )
    ))
    .expect("secrets.generateScript failed, you might need to upgrade bitte");

    let target = format!("{}#{}", flake, instance.flake_attr);
    let cache = format!(
        "{}&secret-key=secrets/nix-secret-key-file",
        instance.s3_cache
    );
    let ip = format!("ssh://root@{}", instance.public_ip);
    let rebuild_flake = format!("{}#{}", flake, instance.uid);

    nix_build(&target);
    nix_copy_to_cache(&target, &cache);
    nix_copy_to_machine(&target, &ip);
    nixos_rebuild(&rebuild_flake, &ip)
}

pub fn nixos_rebuild(target: &String, ip: &String) {
    check_cmd(
        Command::new("nixos-rebuild")
            .arg("switch")
            .arg("--target-host")
            .arg(ip)
            .arg("--flake")
            .arg(target),
    );
}

fn nix_build(target: &String) {
    check_cmd(Command::new("nix").arg("-L").arg("build").arg(target))
}

pub fn nix_copy_to_cache(target: &String, cache: &String) {
    check_cmd(
        Command::new("nix")
            .arg("-L")
            .arg("copy")
            .arg("--to")
            .arg(cache)
            .arg(target),
    );
}

pub fn nix_copy_to_machine(target: &String, ssh: &String) {
    check_cmd(
        Command::new("nix")
            .arg("-L")
            .arg("copy")
            .arg("--substitute-on-destination")
            .arg("--to")
            .arg(ssh)
            .arg(target),
    );
}

pub fn set_ssh_opts(key_checking: bool) -> Result<()> {
    if env::var("NIX_SSHOPTS").is_ok() {
        return Ok(());
    }

    let check = if key_checking { "accept-new" } else { "none" };
    let check_flag = format!("StrictHostKeyChecking={}", check);

    let mut args = vec![
        "-C", // Requests compression of all data
        "-o",
        "NumberOfPasswordPrompts=0",
        "-o",
        "ServerAliveInterval=60",
        "-o",
        "ControlPersist=600",
        "-o",
        &check_flag,
    ];

    let ssh_key_path = format!("secrets/ssh-{}", bitte_cluster()?);
    let ssh_key = Path::new(&ssh_key_path);
    if ssh_key.is_file() {
        args.push("-i");
        args.push(ssh_key_path.as_str());
    }

    env::set_var("NIX_SSHOPTS", args.join(" "));
    Ok(())
}
