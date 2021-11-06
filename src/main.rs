mod cli;
mod utils;

use anyhow::{bail, Result};
use clap::{App, IntoApp};
use cli::opts::Bitte;
use std::env;
use utils::types::BitteCluster;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    let _toml = include_str!("../Cargo.toml");

    let mut app: App = <Bitte as IntoApp>::into_app()
        .name(clap::crate_name!())
        .version(clap::crate_version!())
        .author(clap::crate_authors!("\n"))
        .about(clap::crate_description!());

    let mut help_text = Vec::new();
    app.write_help(&mut help_text)
        .expect("Failed to write help text to buffer");

    let matches = app.get_matches();

    let token: Option<Uuid> = matches.value_of_t("nomad").ok();

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
