mod cli;

use std::io;

use anyhow::{bail, Result};
use clap::{clap_app, crate_authors, crate_version, App, ArgMatches};
use clap_generate::{generate, generators};

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let app = make_app();
    let matches = app.get_matches();

    match matches.subcommand() {
        Some(("plan", sub)) => cli::plan(sub).await,
        Some(("events", sub)) => cli::events(sub).await,
        Some(("completions", sub)) => completions(sub).await,
        _ => bail!("Unknown command"),
    }?;
    Ok(())
}

fn make_app() -> App<'static> {
    clap_app!(iogo =>
      (version: crate_version!())
      (author: crate_authors!())
      (about: "Deploy all the things!")
      (@subcommand events => (@arg foo: +takes_value "foo"))
      (@subcommand plan =>
        (about: "Plan and execute a Nomad job")
        (@arg namespace: +takes_value +required "Name of the namespace")
        (@arg job: +takes_value "Name of the job to run"))
      (@subcommand completions =>
        (about: "Generate shell completion files")
        (@arg shell: +takes_value +required "bash, elvish, fish, powershell, or zsh"))
    )
}

async fn completions(sub: &ArgMatches) -> Result<()> {
    let shell: String = sub.value_of_t_or_exit("shell");
    let mut app = make_app();

    let result = match shell.as_str() {
        "bash" => generate::<generators::Bash, _>(&mut app, "iogo", &mut io::stdout()),
        "elvish" => generate::<generators::Elvish, _>(&mut app, "iogo", &mut io::stdout()),
        "fish" => generate::<generators::Fish, _>(&mut app, "iogo", &mut io::stdout()),
        "powershell" => generate::<generators::PowerShell, _>(&mut app, "iogo", &mut io::stdout()),
        "zsh" => generate::<generators::Zsh, _>(&mut app, "iogo", &mut io::stdout()),
        other => {
            bail!("Unknown shell: {}", other)
        }
    };

    Ok(result)
}
