mod cli;

use anyhow::{bail, Context, Result};
use bitte_lib::types::BitteCluster;
use clap::clap_app;
use clap::{Arg, IntoApp};
use deploy::cli::Opts;
use std::env;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    let _toml = include_str!("../Cargo.toml");

    let mut app = clap_app!((clap::crate_name!()) =>
      (version: clap::crate_version!())
      (author: clap::crate_authors!("\n"))
      (about: clap::crate_description!())
      (@arg provider: --provider<NAME> env[BITTE_PROVIDER] "The cluster infrastructure provider")
      (@arg domain: --domain<NAME> env[BITTE_DOMAIN] "The public domain of the cluster")
      (@arg name: --cluster<NAME> env[BITTE_CLUSTER] "The unique name of the cluster")
      (@arg "nomad-token": --nomad<TOKEN> env[NOMAD_TOKEN] "The Nomad token used to query node information")
      (@subcommand rebuild =>
        (about: "nixos-rebuild")
        (@arg only: -o --only +takes_value +multiple "pattern of hosts to deploy")
        (@arg clients: -l --clients conflicts_with[only] "rebuild all nomad client nodes")
        (@arg delay: -d --delay +takes_value "seconds to delay between rebuilds")
        (@arg copy: -c --copy "copy to the S3 cache first"))
      (@subcommand info =>
        (about: "Show information about instances and auto-scaling groups")
        (@arg json: -j --json "format as json"))
      (@subcommand ssh =>
        (about: "SSH to instances")
        (@arg job: -j --job +takes_value +multiple #{3, 3} "specify client by: job group alloc_index\nauto 'cd' to alloc dir when <args> are not specified")
        (@group multi =>
            (@arg all: -a --all conflicts_with[job] requires[args] "run <args> on all nodes")
            (@arg parallel: -p --parallel conflicts_with[job] requires[args] conflicts_with[all] "run <args> on nodes in parallel"))
        (@arg namespace: -n --namespace +takes_value env[NOMAD_NAMESPACE] "specify nomad namespace to search for <job>\nonly valid with --job flag")
        (@arg clients: -l --clients requires[multi] "for -a and -p, execute commands only on Nomad clients")
        (@arg delay: -d --delay +takes_value requires[all] "for -a, seconds to delay between commands")
        (@arg args: +takes_value +multiple "arguments to ssh"))
      (@subcommand terraform =>
        (about: "Run terraform")
        (aliases: &["tf"])
        (@arg workspace: +takes_value +required "name of the terraform workspace")
        (@subcommand plan => (about: "terraform plan")
          (@arg destroy: --destroy -d "create a destruction plan"))
        (@subcommand apply => (about: "terraform apply"))
        (@subcommand passthrough =>
          (about: "delegate to terraform")
          (aliases: &["passthru", "pt"])
          (@arg no_config: --no-config -n "skip regenerating the terraform config")
          (@arg init: --init -i "delete and reinitialize the `.terraform` state dir before \
          delegating to terraform")
          (@arg args: +takes_value +multiple "arguments to terraform"))
        (@subcommand init => (about: "terraform init")
          (@arg upgrade: --upgrade -u "upgrade provider versions"))
        (@subcommand output => (about: "terraform output")))
      (@subcommand provision =>
        (about: "Initial provisioning from Terraform (do not run yourself)")
        (@arg ip: +takes_value +required "ip of the node")
        (@arg name: +takes_value +required "name of the node")
        (@arg cluster: +takes_value +required "cluster name")
        (@arg flake: +takes_value +required "flake location")
        (@arg attr: +takes_value +required "flake host attr")
        (@arg cache: +takes_value +required "cache location"))
      (@subcommand certs =>
        (@arg domain: +takes_value +required "FQDN of the cluster"))
    )
    .subcommand(<Opts as IntoApp>::into_app().name("deploy"))
    .arg(
        Arg::new("aws-region")
        .about("The default AWS region")
        .long("aws-region")
        .takes_value(true)
        .required_if_eq("provider", "AWS")
        .env("AWS_DEFAULT_REGION")
    ).arg(
        Arg::new("aws-asg-regions")
        .about("Regions containing Nomad clients")
        .long("aws-asg-regions")
        .value_delimiter(":")
        .require_delimiter(true)
        .required_if_eq("provider", "AWS")
        .env("AWS_ASG_REGIONS")
    );

    let mut help_text = Vec::new();
    app.write_help(&mut help_text)
        .expect("Failed to write help text to buffer");

    let matches = app.get_matches();

    let token: Uuid = matches
        .value_of_t("nomad-token")
        .with_context(|| "A Nomad token should be a valid UUID")?;

    let run = |init_log: bool| {
        if init_log {
            pretty_env_logger::init()
        };
        BitteCluster::init(matches.clone(), token)
    };

    match matches.subcommand() {
        Some(("rebuild", sub)) => cli::rebuild(sub, run(true)).await,
        Some(("deploy", sub)) => cli::deploy(sub, run(false)).await,
        Some(("info", sub)) => cli::info(sub, run(true)).await,
        Some(("ssh", sub)) => cli::ssh(sub, run(true)).await,
        Some(("terraform", sub)) => cli::terraform(sub, run(true)).await,
        Some(("provision", sub)) => {
            pretty_env_logger::init();
            cli::provision(sub, matches.value_of_t("name")?).await
        }
        Some(("certs", sub)) => {
            pretty_env_logger::init();
            cli::certs(sub).await
        }
        _ => {
            bail!(format!(
                "Invalid subcommand\n {}",
                String::from_utf8(help_text).expect("help text contains invalid UTF8")
            ))
        }
    }?;
    Ok(())
}
