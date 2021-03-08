mod cli;

use anyhow::{bail, Result};
use clap::clap_app;

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    let matches = clap_app!(bitte =>
      (version: "0.0.1")
      (author: "Michael Fellinger <michael.fellinger@iohk.io>")
      (about: "Deploy all the things!")
      (@subcommand events => (@arg foo: +takes_value "foo"))
      (@subcommand plan =>
        (@arg namespace: +takes_value +required "Name of the namespace")
        (@arg job: +takes_value +required "Name of the job to run"))
      (@subcommand run =>
        (@arg namespace: +takes_value +required "Name of the namespace")
        (@arg job: +takes_value +required "Name of the job to run")
        (@arg index: +takes_value +required "Job modify index from the plan")
      )
    )
    .get_matches();

    match matches.subcommand() {
        Some(("plan", sub)) => cli::plan(sub).await,
        Some(("run", sub)) => cli::run(sub).await,
        Some(("events", sub)) => cli::events(sub).await,
        _ => bail!("Unknown command"),
    }?;
    Ok(())
}
