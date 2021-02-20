use bitte_lib::*;
use std::{time::Duration, env, path::Path, process::Command};
use clap::ArgMatches;
use prettytable::{Table, row, cell};
use log::*;

pub(crate) async fn certs(sub: &ArgMatches) {
    let domain: String = sub.value_of_t_or_exit("domain");
    env::set_var("VAULT_ADDR", format!("https://vault.{}", domain));
    env::set_var("VAULT_CACERT", "secrets/ca.pem");
    env::set_var("VAULT_FORMAT", "json");
    env::set_var("VAULT_SKIP_VERIFY", "true");

    certs::vault_login();
    certs::write_issuing_ca(&domain);
    certs::sign_intermediate();
}

pub(crate) async fn provision(sub: &ArgMatches) {
    // Why can the compiler infer these...
    let ip = sub.value_of_t_or_exit("ip");
    let cluster = sub.value_of_t_or_exit("cluster");
    // ...but not these?
    let flake: String = sub.value_of_t_or_exit("flake");
    let attr: String =  sub.value_of_t_or_exit("attr");
    let cache: String = sub.value_of_t_or_exit("cache");

    rebuild::set_ssh_opts(false);
    ssh::wait_for_ssh(&ip).await;
    ssh::wait_for_ready(&cluster, &ip);
    ssh::ssh_keygen(&ip);

    let toplevel = format!(
        "{}#nixosConfigurations.{}.config.system.build.toplevel",
        flake, attr
    );
    let cache = format!("{}&secret-key=secrets/nix-secret-key-file", &cache);
    let flake = format!("{}#{}", &flake, &attr);
    rebuild::nix_copy_to_cache(&toplevel, &cache);
    rebuild::nix_copy_to_machine(&toplevel, &ip);
    rebuild::nixos_rebuild(&flake, &ip);
}

pub(crate) async fn ssh(sub: &ArgMatches) {
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

pub(crate) async fn rebuild(sub: &ArgMatches) {
    let only: Vec<String> = sub.values_of_t("only").unwrap_or(vec![]);
    let delay = Duration::from_secs(sub.value_of_t::<u64>("delay").unwrap_or(0));

    rebuild::set_ssh_opts(true);
    rebuild::copy(only.iter().map(|o| o.as_str()).collect(), delay).await;
}

pub(crate) async fn info(_sub: &ArgMatches) {
    let info = fetch_current_state_version("clients")
        .or_else(|_| fetch_current_state_version("core"))
        .expect("Coudln't fetch clients or core workspaces");
    info_print(info).await;
}

pub(crate) async fn terraform(sub: &ArgMatches) {
    let workspace: String = sub.value_of_t_or_exit("workspace");

    match sub.subcommand() {
        Some(("plan", sub_sub)) => terraform_plan(workspace, sub_sub).await,
        Some(("apply", sub_sub)) => terraform_apply(workspace, sub_sub).await,
        Some(("workspaces", sub_sub)) => terraform_workspaces(workspace, sub_sub).await,
        _ => println!("Unknown command"),
    }
}

/// Run `terraform plan` in a workspace
///
/// # Arguments
///
/// * `workspace` - a string that holds the name of a terraform workspace
/// * `sub` - `&ArgMatches` holding additional cli flags
///
/// # Examples
///
/// ```
/// terraform_plan("network", arg_matches);
/// ```
pub async fn terraform_plan(workspace: String, sub: &ArgMatches) {
    let destroy: bool = sub.is_present("destroy");
    let plan_file = format!("{}.plan", workspace);

    info!("Plan file: {:?}", plan_file);

    terraform::prepare(workspace);

    let mut cmd = Command::new("terraform");
    let mut full = cmd.arg("plan").arg("-out").arg(plan_file);
    if destroy {
        full = full.arg("-destroy");
    }

    info!("run: {:?}", full);
    full.status()
        .expect(format!("failed to run: {:?}", full).as_str());
}

pub async fn terraform_apply(workspace: String, _sub: &ArgMatches) {
    let plan_file = format!("{}.plan", workspace);
    info!("Plan file: {:?}", plan_file);

    terraform::prepare(workspace);

    let mut cmd = Command::new("terraform");
    let full = cmd.arg("apply").arg(plan_file);

    debug!("run: {:?}", full);
    full.status()
        .expect(format!("failed to run: {:?}", full).as_str());
}

pub async fn terraform_workspaces(_workspace: String, _sub: &ArgMatches) {
    let list = terraform::workspace_list();
    println!("{:?}", list)
}

async fn info_print(current_state_version: String) {
    let output = current_state_version_output(&current_state_version).unwrap();

    let mut instance_table = Table::new();
    instance_table.add_row(row!["Name", "Type", "FlakeAttr", "Private IP", "Public IP"]);

    for (key, val) in output.instances.iter() {
        instance_table.add_row(row![
            key,
            val.instance_type,
            val.flake_attr,
            val.private_ip,
            val.public_ip,
        ]);
    }

    instance_table.printstd();

    if let Some(asgs) = output.asgs {
        let mut asg_table = Table::new();

        asg_table.add_row(row![
            "Id",
            "Type",
            "AZ",
            "State",
            "Status",
            "Protected",
            "PrivateIp",
            "PublicIp"
        ]);

        for (_key, val) in asgs.iter() {
            let info = info::asg_info(val.arn.as_str(), val.region.as_str()).await;
            for asgi in info {
                // TODO: rewrite to take all required instance ids as argument to save time
                let ii = info::instance_info(asgi.instance_id.as_str(), val.region.as_str()).await;
                let iii = ii[0].clone();

                asg_table.add_row(row![
                    asgi.instance_id,
                    asgi.instance_type.unwrap_or("".to_string()),
                    asgi.availability_zone,
                    asgi.lifecycle_state,
                    asgi.health_status,
                    asgi.protected_from_scale_in,
                    iii.public_ip_address.unwrap_or("".to_string()),
                    iii.private_ip_address.unwrap_or("".to_string()),
                ]);
                // asg_table.add_row(row![key, val.instance_type, val.flake_attr, val.count,]);
            }
        }

        asg_table.printstd();
    };
}
