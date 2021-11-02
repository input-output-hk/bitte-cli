use anyhow::Result;
use log::info;
use std::{env, net::IpAddr, path::Path, process::Command, time::Duration};

use crate::{
    check_cmd, handle_command_error,
    ssh::wait_for_ssh,
    types::{BitteCluster, BitteFind, BitteNode},
};

pub async fn copy(
    only: Vec<&str>,
    delay: Duration,
    clients: bool,
    cluster: BitteCluster,
) -> Result<()> {
    info!("only: {:?}", only);

    let instances = if only.is_empty() {
        if clients {
            cluster
                .nodes
                .into_iter()
                .filter(|node| node.nomad_client.is_some())
                .collect()
        } else {
            cluster.nodes
        }
    } else {
        cluster.nodes.find_needles(only)
    };

    let mut iter = instances.iter().peekable();

    while let Some(instance) = iter.next() {
        info!("rebuild: {}, {}", instance.name, instance.pub_ip);
        wait_for_ssh(&instance.pub_ip).await?;
        copy_to(instance, 10)?;
        if iter.peek().is_some() {
            tokio::time::sleep(delay).await;
        }
    }

    Ok(())
}

fn copy_to(instance: &BitteNode, _attempts: u64) -> Result<()> {
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

pub fn set_ssh_opts(key_checking: bool, cluster: &str) -> Result<()> {
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

    let ssh_key_path = format!("secrets/ssh-{}", cluster);
    let ssh_key = Path::new(&ssh_key_path);
    if ssh_key.is_file() {
        args.push("-i");
        args.push(ssh_key_path.as_str());
    }

    env::set_var("NIX_SSHOPTS", args.join(" "));
    Ok(())
}
