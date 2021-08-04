use anyhow::{anyhow, Context, Result};
use bitte_lib::{
    bitte_cluster, certs, find_instance, rebuild, ssh, terraform, types::ClusterHandle,
};
use clap::ArgMatches;
use deploy::cli;
use log::*;
use prettytable::{cell, row, Table};
use std::{env, io, path::Path, process::Command, time::Duration};

pub(crate) async fn certs(sub: &ArgMatches) -> Result<()> {
    let domain: String = sub.value_of_t_or_exit("domain");
    env::set_var("VAULT_ADDR", format!("https://vault.{}", domain));
    env::set_var("VAULT_CACERT", "secrets/ca.pem");
    env::set_var("VAULT_FORMAT", "json");
    env::set_var("VAULT_SKIP_VERIFY", "true");

    certs::vault_login()?;
    certs::write_issuing_ca(&domain);
    certs::sign_intermediate()?;
    Ok(())
}

pub(crate) async fn provision(sub: &ArgMatches) -> Result<()> {
    let ip: String = sub.value_of_t("ip")?;
    let cluster: String = sub.value_of_t("cluster")?;
    let flake: String = sub.value_of_t_or_exit("flake");
    let attr: String = sub.value_of_t_or_exit("attr");
    let cache: String = sub.value_of_t_or_exit("cache");

    rebuild::set_ssh_opts(false)?;
    ssh::wait_for_ssh(&ip).await?;
    ssh::wait_for_ready(&cluster, &ip)?;
    ssh::ssh_keygen(&ip)?;

    let toplevel = format!(
        "{}#nixosConfigurations.{}.config.system.build.toplevel",
        flake, attr
    );
    let cache = format!("{}&secret-key=secrets/nix-secret-key-file", &cache);
    let flake = format!("{}#{}", &flake, &attr);
    rebuild::nix_copy_to_cache(&toplevel, &cache)?;
    rebuild::nix_copy_to_machine(&toplevel, &ip)?;
    rebuild::nixos_rebuild(&flake, &ip)?;
    Ok(())
}

pub(crate) async fn ssh(sub: &ArgMatches) -> Result<()> {
    let needle: String = sub.value_of_t("host")?;
    let mut args = sub.values_of_lossy("args").unwrap_or_default();

    let ip = find_instance(needle.as_str())
        .await
        .map_or_else(|| needle.clone(), |i| i.public_ip);
    let user_host = format!("root@{}", ip);
    let mut flags = vec!["-x".to_string(), "-p".into(), "22".into()];

    let ssh_key_path = format!("secrets/ssh-{}", bitte_cluster()?);
    let ssh_key = Path::new(&ssh_key_path);
    if ssh_key.is_file() {
        flags.push("-i".to_string());
        flags.push(ssh_key_path.to_string());
    }

    flags.push(user_host);
    flags.append(args.as_mut());

    if !args.is_empty() {
        flags.append(&mut vec!["-t".to_string()]);
    }
    let ssh_args = flags.into_iter();

    let mut cmd = Command::new("ssh");
    let cmd_with_args = cmd.args(ssh_args);
    println!("cmd: {:?}", cmd_with_args);

    cmd.spawn()
        .context("ssh command failed")?
        .wait()
        .context("ssh command didn't finish?")?;
    Ok(())
}

pub(crate) async fn rebuild(sub: &ArgMatches) -> Result<()> {
    let only: Vec<String> = sub.values_of_t("only").unwrap_or_default();
    let delay = Duration::from_secs(sub.value_of_t::<u64>("delay").unwrap_or(0));
    let copy: bool = sub.value_of_t("copy").unwrap_or(false);

    rebuild::set_ssh_opts(true)?;
    rebuild::copy(only.iter().map(|o| o.as_str()).collect(), delay, copy).await?;
    Ok(())
}
pub(crate) async fn deploy(sub: &ArgMatches) -> Result<()> {
    match cli::run(Some(sub)).await {
        Ok(()) => (),
        Err(err) => {
            error!("{}", err);
            std::process::exit(1);
        }
    }
    Ok(())
}

pub(crate) async fn info(sub: &ArgMatches, cluster: ClusterHandle) -> Result<()> {
    let json: bool = sub.is_present("json");
    info_print(cluster, json).await?;
    Ok(())
}

pub(crate) async fn terraform(sub: &ArgMatches) -> Result<()> {
    let workspace: String = sub.value_of_t_or_exit("workspace");

    match sub.subcommand() {
        Some(("plan", sub_sub)) => terraform_plan(workspace, sub_sub).await,
        Some(("apply", sub_sub)) => terraform_apply(workspace, sub_sub).await,
        Some(("init", sub_sub)) => terraform_init(workspace, sub_sub).await,
        Some(("passthrough", sub_sub)) => terraform_passthrough(workspace, sub_sub).await,
        Some(("output", sub_sub)) => terraform_output(workspace, sub_sub).await,
        _ => Err(anyhow!("Unknown command")),
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
pub async fn terraform_plan(workspace: String, sub: &ArgMatches) -> Result<()> {
    let destroy: bool = sub.is_present("destroy");
    let plan_file = format!("{}.plan", workspace);

    info!("Plan file: {:?}", plan_file);

    terraform::prepare(workspace)?;

    let mut cmd = Command::new("terraform");
    let mut full = cmd.arg("plan").arg("-out").arg(plan_file);
    if destroy {
        full = full.arg("-destroy");
    }

    info!("run: {:?}", full);
    full.status()
        .with_context(|| format!("failed to run: {:?}", full))?;
    Ok(())
}

/// Run any terraform command in a workspace
///
/// # Arguments
///
/// * `workspace` - a string that holds the name of a terraform workspace
/// * `sub` - `&ArgMatches` holding additional cli flags
///
/// # Examples
///
/// ```
/// terraform_passthrough("core", arg_matches);
/// ```
pub async fn terraform_passthrough(workspace: String, sub: &ArgMatches) -> Result<()> {
    let init: bool = sub.is_present("init");
    let config: bool = !sub.is_present("no_config");
    let args = sub.values_of_lossy("args").unwrap_or_default();

    terraform::set_http_auth()?;

    if config {
        terraform::generate_terraform_config(&workspace)?;
    }

    if init {
        terraform::init(false)?;
    }

    let mut cmd = Command::new("terraform");
    let full = cmd.args(args);

    info!("run: {:?}", full);
    full.status()
        .with_context(|| format!("failed to run: {:?}", full))?;
    Ok(())
}

pub async fn terraform_init(workspace: String, sub: &ArgMatches) -> Result<()> {
    let upgrade: bool = sub.is_present("upgrade");
    terraform::generate_terraform_config(&workspace)?;
    terraform::init(upgrade)?;
    Ok(())
}

pub async fn terraform_output(workspace: String, _sub: &ArgMatches) -> Result<()> {
    let output = terraform::output(workspace.as_str())?;
    println!("{:?}", output);
    Ok(())
}

pub async fn terraform_apply(workspace: String, _sub: &ArgMatches) -> Result<()> {
    let plan_file = format!("{}.plan", workspace);
    info!("Plan file: {:?}", plan_file);

    terraform::prepare(workspace)?;

    let mut cmd = Command::new("terraform");
    let full = cmd.arg("apply").arg(plan_file);

    debug!("run: {:?}", full);
    full.status()
        .with_context(|| format!("failed to run: {:?}", full))?;
    Ok(())
}

async fn info_print(cluster: ClusterHandle, json: bool) -> Result<()> {
    if json {
        let stdout = io::stdout();
        let handle = stdout.lock();
        serde_json::to_writer(handle, &cluster.await??)?;
    } else {
        let mut instance_table = Table::new();
        instance_table.add_row(row!["Name", "Private IP", "Public IP"]);

        let nodes = cluster.await??.nodes;

        for node in nodes.into_iter() {
            let mut name = if node.nomad_client.is_some() {
                node.nomad_client.unwrap().id.to_hyphenated().to_string()
            } else {
                node.name.unwrap_or_default()
            };
            if name.is_empty() {
                name = node.nixos.unwrap_or_default();
            }
            instance_table.add_row(row![name, node.priv_ip.unwrap(), node.pub_ip.unwrap()]);
        }

        instance_table.printstd();
    }

    Ok(())
}
