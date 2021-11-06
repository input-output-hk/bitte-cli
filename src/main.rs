mod cli;
mod utils;

use anyhow::{bail, Context, Result};
use clap::{App, Arg, IntoApp};
use deploy::cli::Opts;
use std::env;
use utils::types::BitteCluster;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    let _toml = include_str!("../Cargo.toml");

    let mut app = App::new(clap::crate_name!())
        .version(clap::crate_version!())
        .author(clap::crate_authors!("\n"))
        .about(clap::crate_description!())
        .arg(
            Arg::new("provider")
                .about("The cluster infrastructure provider")
                .env("BITTE_PROVIDER")
                .long("provider")
                .takes_value(true)
                .required(true)
                .value_name("NAME"),
        )
        .arg(
            Arg::new("domain")
                .about("The public domain of the cluster")
                .env("BITTE_DOMAIN")
                .long("domain")
                .takes_value(true)
                .required(true)
                .value_name("NAME"),
        )
        .arg(
            Arg::new("name")
                .about("The unique name of the cluster")
                .env("BITTE_CLUSTER")
                .long("cluster")
                .takes_value(true)
                .required(true)
                .value_name("NAME"),
        )
        .arg(
            Arg::new("nomad")
                .about("The Nomad token used to query node information")
                .env("NOMAD_TOKEN")
                .long("nomad")
                .takes_value(true)
                .global(true)
                .value_name("TOKEN"),
        )
        .subcommand(
            App::new("info")
                .about("Show information about instances and auto-scaling groups")
                .arg(
                    Arg::new("json")
                        .short('j')
                        .long("json")
                        .about("Show information about instances and auto-scaling groups"),
                ),
        )
        .subcommand(
            App::new("ssh")
                .about("SSH to instances")
                .arg(
                    Arg::new("job")
                        .about("specify client by: job, group, alloc_index\nauto 'cd' to alloc dir when <args> are not specified")
                        .short('j')
                        .long("job")
                        .requires("nomad")
                        .number_of_values(3)
                        .value_names(&["job", "group", "index"])
                ),
        )
        //   (@subcommand ssh =>
        //     (about: "SSH to instances")
        //     (@arg job: -j --job requires[nomad] +takes_value +multiple #{3, 3} "specify client by: job group alloc_index\nauto 'cd' to alloc dir when <args> are not specified")
        //     (@group multi =>
        //         (@arg all: -a --all conflicts_with[job] requires[args] "run <args> on all nodes")
        //         (@arg parallel: -p --parallel conflicts_with[job] requires[args] conflicts_with[all] "run <args> on nodes in parallel"))
        //     (@arg namespace: -n --namespace +takes_value env[NOMAD_NAMESPACE] "specify nomad namespace to search for <job>\nonly valid with --job flag")
        //     (@arg clients: -l --clients requires[multi] "for -a and -p, execute commands only on Nomad clients")
        //     (@arg delay: -d --delay +takes_value requires[all] "for -a, seconds to delay between commands")
        //     (@arg args: +takes_value +multiple "arguments to ssh"))
        //   (@subcommand terraform =>
        //     (about: "Run terraform")
        //     (aliases: &["tf"])
        //     (@arg workspace: +takes_value +required "name of the terraform workspace")
        //     (@subcommand plan => (about: "terraform plan")
        //       (@arg destroy: --destroy -d "create a destruction plan"))
        //     (@subcommand apply => (about: "terraform apply"))
        //     (@subcommand passthrough =>
        //       (about: "delegate to terraform")
        //       (aliases: &["passthru", "pt"])
        //       (@arg no_config: --no-config -n "skip regenerating the terraform config")
        //       (@arg init: --init -i "delete and reinitialize the `.terraform` state dir before \
        //       delegating to terraform")
        //       (@arg args: +takes_value +multiple "arguments to terraform"))
        //     (@subcommand init => (about: "terraform init")
        //       (@arg upgrade: --upgrade -u "upgrade provider versions")))
        //   (@subcommand certs =>
        //     (@arg domain: +takes_value +required "FQDN of the cluster"))
        // )
        .subcommand(<Opts as IntoApp>::into_app().name("deploy"))
        .arg(
            Arg::new("aws-region")
                .about("The default AWS region")
                .long("aws-region")
                .takes_value(true)
                .required_if_eq("provider", "AWS")
                .env("AWS_DEFAULT_REGION"),
        )
        .arg(
            Arg::new("aws-asg-regions")
                .about("Regions containing Nomad clients")
                .long("aws-asg-regions")
                .value_delimiter(':')
                .require_delimiter(true)
                .required_if_eq("provider", "AWS")
                .env("AWS_ASG_REGIONS"),
        );

    let mut help_text = Vec::new();
    app.write_help(&mut help_text)
        .expect("Failed to write help text to buffer");

    let matches = app.get_matches();

    let token: Option<Uuid> = {
        if matches.is_present("nomad") {
            let token = matches
                .value_of_t("nomad")
                .with_context(|| "A Nomad token should be a valid UUID")?;
            Some(token)
        } else {
            None
        }
    };

    let run = |init_log: bool| {
        if init_log {
            pretty_env_logger::init()
        };
        BitteCluster::init(matches.clone(), token)
    };

    match matches.subcommand() {
        Some(("deploy", sub)) => cli::deploy(sub, run(false)).await,
        Some(("info", sub)) => cli::info(sub, run(true)).await,
        Some(("ssh", sub)) => cli::ssh(sub, run(true)).await,
        Some(("terraform", sub)) => cli::terraform(sub, run(true)).await,
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
