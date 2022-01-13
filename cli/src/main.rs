mod cli;
mod types;
mod utils;

use anyhow::Result;
use clap::{App, IntoApp};
use cli::opts::Bitte;
use types::BitteCluster;

#[tokio::main]
async fn main() -> Result<()> {
    let _toml = include_str!("../Cargo.toml");

    let app: App = <Bitte as IntoApp>::into_app();

    let matches = app.clone().get_matches();

    let run = |init_log: bool| {
        if init_log {
            cli::init_log(matches.occurrences_of("verbose"))
        };
        BitteCluster::init(matches.clone())
    };

    match matches.subcommand() {
        Some(("deploy", sub)) => cli::deploy(sub, run(false)).await?,
        Some(("info", sub)) => cli::info(sub, run(true)).await?,
        Some(("ssh", sub)) => cli::ssh(sub, run(true)).await?,
        Some(("completions", sub)) => cli::completions(sub, app).await?,
        _ => (),
    }
    Ok(())
}
