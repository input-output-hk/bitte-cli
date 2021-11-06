mod args;
pub mod opts;
pub mod subs;

use crate::utils::{
    terraform,
    types::{BitteFind, ClusterHandle},
};
use anyhow::{anyhow, Context, Result};
use clap::ArgMatches;
use deploy::cli;
use log::*;
use prettytable::{cell, row, Table};
use std::net::IpAddr;
use std::{env, io, path::Path, process::Command, time::Duration};
use tokio::task::JoinHandle;

pub(crate) async fn ssh(sub: &ArgMatches, cluster: ClusterHandle) -> Result<()> {
    let mut args = sub.values_of_lossy("args").unwrap_or_default();
    let job: Vec<String> = sub.values_of_t("job").unwrap_or_default();
    let delay = Duration::from_secs(sub.value_of_t::<u64>("delay").unwrap_or(0));

    let namespace: String = sub.value_of_t("namespace").unwrap_or_default();

    let ip: IpAddr;

    let cluster = cluster.await??;

    if sub.is_present("all") {
        let nodes = if sub.is_present("clients") {
            cluster
                .nodes
                .into_iter()
                .filter(|node| node.nomad_client.is_some())
                .collect()
        } else {
            cluster.nodes
        };

        let mut iter = nodes.iter().peekable();

        while let Some(node) = iter.next() {
            init_ssh(node.pub_ip, args.clone(), cluster.name.clone()).await?;
            if sub.is_present("delay") && iter.peek().is_some() {
                tokio::time::sleep(delay).await;
            }
        }

        return Ok(());
    } else if sub.is_present("parallel") {
        let nodes = if sub.is_present("clients") {
            cluster
                .nodes
                .into_iter()
                .filter(|node| node.nomad_client.is_some())
                .collect()
        } else {
            cluster.nodes
        };

        let mut handles: Vec<JoinHandle<Result<()>>> = Vec::with_capacity(nodes.len());

        for node in nodes.into_iter() {
            let args = args.clone();
            let name = cluster.name.clone();
            let handle = tokio::spawn(async move { init_ssh(node.pub_ip, args, name).await });
            handles.push(handle);
        }

        for handle in handles.into_iter() {
            handle.await??;
        }

        return Ok(());
    } else if sub.is_present("job") {
        let (name, group, index) = (&*job[0], &*job[1], &job[2]);

        let nodes = cluster.nodes;
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

                allocs.as_ref().unwrap().iter().any(|alloc| {
                    let is_alloc = alloc.namespace == namespace
                        && alloc.job_id == name
                        && alloc.task_group == group
                        && alloc.index.get() == index.parse().ok()
                        && alloc.status == "running";

                    if is_alloc && args.is_empty() {
                        args.extend(vec![
                            "-t".into(),
                            format!("cd /var/lib/nomad/alloc/{}; bash", alloc.id),
                        ]);
                    }

                    is_alloc
                })
            })
            .with_context(|| {
                format!(
                    "{}, {}, {} does not match any nomad allocations",
                    name, group, index
                )
            })?;

        ip = node.pub_ip;
    } else {
        let needle = args.first();

        if needle.is_none() {
            return Err(anyhow!("first arg must be a host"));
        }

        let needle = needle.unwrap().clone();
        args = args.drain(1..).collect();

        let nodes = cluster.nodes;
        let node = nodes.find_needle(&needle)?;

        ip = node.pub_ip;
    };

    init_ssh(ip, args, cluster.name).await
}

async fn init_ssh(ip: IpAddr, args: Vec<String>, cluster: String) -> Result<()> {
    let user_host = &*format!("root@{}", ip);
    let mut flags = vec!["-x", "-p", "22"];

    let ssh_key_path = format!("secrets/ssh-{}", cluster);
    let ssh_key = Path::new(&ssh_key_path);
    if ssh_key.is_file() {
        flags.push("-i");
        flags.push(&*ssh_key_path);
    }

    flags.append(&mut vec!["-o", "StrictHostKeyChecking=accept-new"]);

    flags.push(user_host);

    if !args.is_empty() {
        flags.append(&mut args.iter().map(|string| string.as_str()).collect());
    }
    let ssh_args = flags.into_iter();

    let mut cmd = Command::new("ssh");
    let cmd_with_args = cmd.args(ssh_args);
    info!("cmd: {:?}", cmd_with_args);

    cmd.spawn()
        .with_context(|| "ssh command failed")?
        .wait()
        .with_context(|| "ssh command didn't finish?")?;
    Ok(())
}

pub(crate) async fn deploy(sub: &ArgMatches, cluster: ClusterHandle) -> Result<()> {
    cluster.await??;
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

pub(crate) async fn terraform(sub: &ArgMatches, cluster: ClusterHandle) -> Result<()> {
    let workspace: String = sub.value_of_t_or_exit("workspace");

    match sub.subcommand() {
        Some(("plan", sub_sub)) => terraform_plan(workspace, sub_sub, cluster).await,
        Some(("apply", sub_sub)) => terraform_apply(workspace, sub_sub, cluster).await,
        Some(("init", sub_sub)) => terraform_init(workspace, sub_sub, cluster).await,
        Some(("passthrough", sub_sub)) => terraform_passthrough(workspace, sub_sub, cluster).await,
        _ => {
            cluster.abort();
            Err(anyhow!("Unknown command"))
        }
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
pub async fn terraform_plan(
    workspace: String,
    sub: &ArgMatches,
    cluster: ClusterHandle,
) -> Result<()> {
    let destroy: bool = sub.is_present("destroy");
    let plan_file = format!("{}.plan", workspace);

    info!("Plan file: {:?}", plan_file);

    terraform::prepare(workspace, cluster).await?;

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
pub async fn terraform_passthrough(
    workspace: String,
    sub: &ArgMatches,
    cluster: ClusterHandle,
) -> Result<()> {
    let init: bool = sub.is_present("init");
    let config: bool = !sub.is_present("no_config");
    let args = sub.values_of_lossy("args").unwrap_or_default();

    terraform::set_http_auth()?;

    if config {
        terraform::generate_terraform_config(&workspace, cluster).await?;
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

pub async fn terraform_init(
    workspace: String,
    sub: &ArgMatches,
    cluster: ClusterHandle,
) -> Result<()> {
    let upgrade: bool = sub.is_present("upgrade");
    terraform::generate_terraform_config(&workspace, cluster).await?;
    terraform::init(upgrade)?;
    Ok(())
}

pub async fn terraform_apply(
    workspace: String,
    _sub: &ArgMatches,
    cluster: ClusterHandle,
) -> Result<()> {
    let plan_file = format!("{}.plan", workspace);
    info!("Plan file: {:?}", plan_file);

    terraform::prepare(workspace, cluster).await?;

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
        let cluster = cluster.await??;
        env::set_var("BITTE_INFO_NO_ALLOCS", "");
        serde_json::to_writer_pretty(handle, &cluster)?;
    } else {
        let mut instance_table = Table::new();
        instance_table.add_row(row![
            "Name",
            "Private IP",
            "Public IP",
            "Type",
            "Zone",
            "Suffix"
        ]);

        let nodes = cluster.await??.nodes;

        for node in nodes.into_iter() {
            let name = if node.nomad_client.is_some() {
                node.nomad_client.unwrap().id.to_hyphenated().to_string()
            } else {
                node.name
            };

            let suffix = {
                let asg = node
                    .asg
                    .unwrap_or_default()
                    .split('-')
                    .last()
                    .unwrap_or_default()
                    .to_owned();

                let i_type = node
                    .node_type
                    .clone()
                    .unwrap_or_default()
                    .split('.')
                    .last()
                    .unwrap_or_default()
                    .to_owned();
                if !asg.is_empty() && asg != i_type {
                    Some(asg)
                } else {
                    None
                }
            };

            instance_table.add_row(row![
                name,
                node.priv_ip,
                node.pub_ip,
                node.node_type.unwrap_or_default(),
                node.zone.unwrap_or_default(),
                suffix.unwrap_or_default()
            ]);
        }

        instance_table.printstd();
    }

    Ok(())
}
