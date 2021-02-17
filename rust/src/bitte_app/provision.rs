use std::{path::Path, process::Command};

use clap::ArgMatches;

use super::{
    check_cmd,
    rebuild::{nix_copy_to_cache, nix_copy_to_machine, nixos_rebuild, set_ssh_opts},
    wait_for_ssh,
};

struct Args {
    ip: String,
    cluster: String,
    flake: String,
    attr: String,
    cache: String,
}

pub async fn cli_provision(sub: &ArgMatches) {
    let args = &Args {
        ip: sub.value_of_t_or_exit("ip"),
        cluster: sub.value_of_t_or_exit("cluster"),
        flake: sub.value_of_t_or_exit("flake"),
        attr: sub.value_of_t_or_exit("attr"),
        cache: sub.value_of_t_or_exit("cache"),
    };

    set_ssh_opts(false);
    wait_for_ssh(args.ip.clone()).await;
    wait_for_ready(args);
    ssh_keygen(&args.ip);

    let toplevel = format!(
        "{}#nixosConfigurations.{}.config.system.build.toplevel",
        args.flake, args.attr
    );
    let cache = format!("{}&secret-key=secrets/nix-secret-key-file", args.cache);
    let flake = format!("{}#{}", args.flake, args.attr);
    nix_copy_to_cache(&toplevel, cache);
    nix_copy_to_machine(&toplevel, &args.ip);
    nixos_rebuild(&flake, &args.ip);
}

fn ssh_keygen(ip: &String) {
    check_cmd(Command::new("ssh-keygen").arg("-R").arg(ip));
}

fn wait_for_ready(args: &Args) {
    let target = format!("root@#{}", args.ip);

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

    let ssh_key_path = format!("secrets/ssh-{}", args.cluster);
    let ssh_key = Path::new(&ssh_key_path);
    if ssh_key.is_file() {
        ssh_args.push("-i");
        ssh_args.push(ssh_key_path.as_str());
    }

    ssh_args.push(&target);
    ssh_args.push("until grep true /etc/ready &>/dev/null; do sleep 1; done");
    check_cmd(Command::new("ssh").args(ssh_args))
}
