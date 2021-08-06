use anyhow::Result;
use log::info;
use std::{env, net::IpAddr, path::Path, process::Command, time::Duration};

use super::{
    bitte_cluster, check_cmd, handle_command_error,
    ssh::wait_for_ssh,
    types::{BitteFind, BitteNode, ClusterHandle},
};

pub async fn copy(
    only: Vec<&str>,
    delay: Duration,
    copy: bool,
    cluster: ClusterHandle,
) -> Result<()> {
    info!("only: {:?}", only);

    let cluster = cluster.await??;
    let instances = if only.is_empty() {
        cluster.nodes
    } else {
        cluster.nodes.find_needles(only)
    };

    let mut iter = instances.iter().peekable();

    let cache = if copy { Some(cluster.s3_cache) } else { None };

    while let Some(instance) = iter.next() {
        info!("rebuild: {}", instance.name);
        wait_for_ssh(&instance.pub_ip).await?;
        copy_to(&instance, 10, &cache)?;
        if iter.peek().is_some() {
            tokio::time::sleep(delay).await;
        }
    }

    Ok(())
}

fn copy_to(instance: &BitteNode, _attempts: u64, cache: &Option<String>) -> Result<()> {
    env::set_var("IP", instance.pub_ip.to_string());
    let flake = ".";

    handle_command_error(execute::command_args!(
        "nix",
        "run",
        format!(
            "{}#nixosConfigurations.{}.config.secrets.generateScript",
            flake, instance.nixos
        )
    ))?;

    let target = format!(
        "{}#nixosConfigurations.{}.config.system.build.toplevel",
        flake, instance.nixos
    );
    let rebuild_flake: String = format!("{}#{}", flake, instance.nixos);

    nix_build(&target)?;

    if let Some(c) = cache {
        let cache = format!("{}&secret-key=secrets/nix-secret-key-file", c);
        nix_copy_to_cache(&target, &cache)?;
    }

    nix_copy_to_machine(&target, &instance.pub_ip)?;
    nixos_rebuild(&rebuild_flake, &instance.pub_ip)
}

pub fn nixos_rebuild(target: &str, ip: &IpAddr) -> Result<()> {
    check_cmd(
        Command::new("nixos-rebuild")
            .arg("switch")
            .arg("--build-host")
            .arg("localhost")
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

pub fn nix_copy_to_machine(target: &str, ssh: &IpAddr) -> Result<()> {
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
