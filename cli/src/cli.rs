use anyhow::{anyhow, Context, Result};
use bitte_lib::{bitte_cluster, certs, rebuild, ssh, terraform, types::ClusterHandle};
use clap::ArgMatches;
use deploy::cli;
use log::*;
use prettytable::{cell, row, Table};
use std::net::{IpAddr, Ipv4Addr};
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

pub(crate) async fn ssh(sub: &ArgMatches, cluster: ClusterHandle) -> Result<()> {
    let mut args = sub.values_of_lossy("args").unwrap_or_default();
    let needle = args.first();
    let job: Vec<String> = sub.values_of_t("job").unwrap_or_default();
    let namespace: String = sub
        .value_of_t("namespace")
        .unwrap_or(env::var("NOMAD_NAMESPACE")?);

    let ip: IpAddr;

    if sub.is_present("job") {
        let name = job.get(0).unwrap();
        let group = job.get(1).unwrap();
        let index = job.get(2).unwrap();

        let nodes = cluster.await??.nodes;
        let node = nodes
            .into_iter()
            .find(|node| {
                let client = &node.nomad_client;
                if client.is_none() {
                    return false;
                };

                let allocs = &client.as_ref().unwrap().allocs;
                if allocs.is_none() || allocs.as_ref().unwrap().is_empty() {
                    return false;
                };

                allocs
                    .as_ref()
                    .unwrap()
                    .into_iter()
                    .find(|alloc| {
                        alloc.namespace == namespace
                            && &alloc.job_id == name
                            && &alloc.task_group == group
                            && Some(alloc.index) == index.parse().ok()
                    })
                    .is_some()
            })
            .with_context(|| {
                format!(
                    "{}, {}, {} does not match any nomad allocations",
                    name, group, index
                )
            })?;

        ip = node
            .pub_ip
            .with_context(|| format!("job {} does not have a public IP address", name))?;
    } else {
        if needle.is_none() {
            return Err(anyhow!("first arg must be a host"));
        }
        let needle = needle.unwrap().clone();
        args = args.drain(1..).collect();
        let nodes = cluster.await??.nodes;
        let node = nodes
            .into_iter()
            .find(|node| {
                let ip = needle.parse::<IpAddr>().ok();
                let empty = &"".to_owned();

                node.id.as_ref().unwrap_or(empty) == &needle
                    || node.name.as_ref().unwrap_or(empty) == &needle
                    || node
                        .nomad_client
                        .as_ref()
                        .unwrap_or(&Default::default())
                        .id
                        .to_hyphenated()
                        .to_string()
                        == *needle
                    || node.priv_ip == ip
                    || node.pub_ip == ip
            })
            .with_context(|| format!("{} does not match any nodes", needle))?;

        ip = node
            .pub_ip
            .with_context(|| format!("{} does not have a public IP address", needle))?
    };

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
    info!("cmd: {:?}", cmd_with_args);

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

            let no_ip = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));

            instance_table.add_row(row![
                name,
                node.priv_ip.unwrap_or(no_ip),
                node.pub_ip.unwrap_or(no_ip)
            ]);
        }

        instance_table.printstd();
    }

    Ok(())
}
