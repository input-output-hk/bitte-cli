mod cli;
mod types;
mod utils;

use anyhow::Result;
use clap::{App, IntoApp};
use cli::opts::Bitte;
use std::env;
use types::BitteCluster;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    let _toml = include_str!("../Cargo.toml");

    let app: App = <Bitte as IntoApp>::into_app()
        .name(clap::crate_name!())
        .version(clap::crate_version!())
        .author(clap::crate_authors!("\n"))
        .about(clap::crate_description!());

    let matches = app.get_matches();

    let token: Option<Uuid> = matches.value_of_t("nomad").ok();

    let run = |init_log: bool| {
        if init_log {
            pretty_env_logger::init()
        };
        BitteCluster::init(matches.clone(), token)
    };

    match matches.subcommand() {
        Some(("deploy", sub)) => cli::deploy(sub, run(false)).await?,
        Some(("info", sub)) => cli::info(sub, run(true)).await?,
        Some(("ssh", sub)) => cli::ssh(sub, run(true)).await?,
        Some(("terraform", sub)) => cli::terraform(sub, run(true)).await?,
        _ => (),
    }
    Ok(())
}
