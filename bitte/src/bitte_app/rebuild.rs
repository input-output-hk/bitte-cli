use std::{env, path::Path, process::Command, time::Duration};

use super::{
    bitte_cluster, check_cmd, current_state_version_output, fetch_current_state_version,
    handle_command_error, types::HttpWorkspaceStateInstance, wait_for_ssh,
};

pub(crate) async fn rebuild_copy(only: &Vec<String>, delay: Duration) {
    let current_state_version =
        fetch_current_state_version("core").expect("terraform state for core is missing");
    let output = current_state_version_output(&current_state_version)
        .expect("Problem loading state version from terraform");

    let instances = output
        .instances
        .values()
        .collect::<Vec<&HttpWorkspaceStateInstance>>()
        .into_iter();

    let mut iter = if only.is_empty() {
        instances.peekable()
    } else {
        instances
            .filter(|instance| {
                only.iter().any(|needle| {
                    &instance.private_ip == needle
                        || &instance.public_ip == needle
                        || &instance.name == needle
                })
            })
            .collect::<Vec<&HttpWorkspaceStateInstance>>()
            .into_iter()
            .peekable()
    };

    while let Some(instance) = iter.next() {
        println!("rebuild: {}", instance.name);
        wait_for_ssh(instance.public_ip.clone()).await;
        copy_to(&instance, output.s3_cache.clone(), 10);
        if iter.peek().is_some() {
            tokio::time::sleep(delay).await;
        }
    }
}

fn copy_to(instance: &HttpWorkspaceStateInstance, s3_cache: String, _attempts: u64) {
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

    let target = &format!("{}#{}", flake, instance.flake_attr);
    let cache = format!("{}&secret-key=secrets/nix-secret-key-file", s3_cache);
    let ip = format!("ssh://root@{}", instance.public_ip);
    let rebuild_flake = format!("{}#{}", flake, instance.uid);

    nix_build(target);
    nix_copy_to_cache(target, cache);
    nix_copy_to_machine(target, &ip);
    nixos_rebuild(&rebuild_flake, &ip)
}

pub(crate) fn nixos_rebuild(target: &String, ip: &String) {
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

pub(crate) fn nix_copy_to_cache(target: &String, cache: String) {
    check_cmd(
        Command::new("nix")
            .arg("-L")
            .arg("copy")
            .arg("--to")
            .arg(cache)
            .arg(target),
    );
}

pub(crate) fn nix_copy_to_machine(target: &String, ssh: &String) {
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

pub(crate) fn set_ssh_opts(key_checking: bool) {
    if env::var("NIX_SSHOPTS").is_ok() {
        return;
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

    let ssh_key_path = format!("secrets/ssh-{}", bitte_cluster());
    let ssh_key = Path::new(&ssh_key_path);
    if ssh_key.is_file() {
        args.push("-i");
        args.push(ssh_key_path.as_str());
    }

    env::set_var("NIX_SSHOPTS", args.join(" "));
}
