use anyhow::{Context, Result};
use log::info;
use std::{env, path::Path, process::Command, time::Duration};

use super::{
    bitte_cluster, check_cmd, find_instances, handle_command_error, ssh::wait_for_ssh, Instance,
};

pub async fn copy(only: Vec<&str>, delay: Duration, copy: bool) -> Result<()> {
    info!("only: {:?}", only);
    let instances = find_instances(only.clone()).await.into_iter();
    let instance_names: Vec<String> = instances.clone().map(|i| i.name).collect();
    info!("instances: {:?}", instance_names);
    let mut iter = instances.peekable();

    while let Some(instance) = iter.next() {
        info!("rebuild: {}", instance.name);
        wait_for_ssh(&instance.public_ip).await?;
        copy_to(&instance, 10, copy)?;
        if iter.peek().is_some() {
            tokio::time::sleep(delay).await;
        }
    }

    Ok(())
}

fn copy_to(instance: &Instance, _attempts: u64, copy: bool) -> Result<()> {
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
    .context("secrets.generateScript failed, you might need to upgrade bitte")?;

    let target = format!("{}#{}", flake, instance.flake_attr);
    let cache = format!(
        "{}&secret-key=secrets/nix-secret-key-file",
        instance.s3_cache
    );
    let rebuild_flake: String = format!("{}#{}", flake, instance.uid);

    nix_build(&target)?;
    if copy {
        nix_copy_to_cache(&target, &cache)?;
    }
    nix_copy_to_machine(&target, &instance.public_ip)?;
    nixos_rebuild(&rebuild_flake, &instance.public_ip)
}

pub fn nixos_rebuild(target: &str, ip: &str) -> Result<()> {
    check_cmd(
        Command::new("nixos-rebuild")
            .arg("switch")
            .arg("--target-host")
            .arg(format!("root@{}", ip))
            .arg("--flake")
            .arg(target),
    )?;
    Ok(())
}

fn nix_build(target: &str) -> Result<()> {
    check_cmd(Command::new("nix").arg("-L").arg("build").arg(target))?;
    Ok(())
}

pub fn nix_copy_to_cache(target: &str, cache: &str) -> Result<()> {
    check_cmd(
        Command::new("nix")
            .arg("-L")
            .arg("copy")
            .arg("--to")
            .arg(cache)
            .arg(target),
    )?;
    Ok(())
}

pub fn nix_copy_to_machine(target: &str, ssh: &str) -> Result<()> {
    check_cmd(
        Command::new("nix")
            .arg("-L")
            .arg("copy")
            .arg("--substitute-on-destination")
            .arg("--to")
            .arg(format!("ssh://root@{}", ssh))
            .arg(target),
    )?;
    Ok(())
}

pub fn set_ssh_opts(key_checking: bool) -> Result<()> {
    if env::var("NIX_SSHOPTS").is_ok() {
        return Ok(());
    }

    let check = if key_checking { "accept-new" } else { "no" };
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
