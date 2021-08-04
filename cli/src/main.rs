mod cli;

use anyhow::{bail, Result};
use bitte_lib::types::BitteCluster;
use clap::clap_app;
use clap::IntoApp;
use deploy::cli::Opts;

#[tokio::main]
async fn main() -> Result<()> {
    let cluster = tokio::spawn(BitteCluster::new());

    let mut app = clap_app!(bitte =>
      (version: "0.0.1")
      (author: "Michael Fellinger <michael.fellinger@iohk.io>")
      (about: "Deploy all the things!")
      (@subcommand rebuild =>
        (about: "nixos-rebuild")
        (@arg only: --only +takes_value +multiple "pattern of hosts to deploy")
        (@arg delay: --delay +takes_value "seconds to delay between rebuilds")
        (@arg copy: --copy "copy to the S3 cache first"))
      (@subcommand info =>
        (about: "Show information about instances and auto-scaling groups")
        (@arg json: --json "format as json"))
      (@subcommand ssh =>
        (about: "SSH to instances")
        (@arg host: +takes_value "host")
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
    .subcommand(<Opts as IntoApp>::into_app().name("deploy"));

    let mut help_text = Vec::new();
    app.write_help(&mut help_text)
        .expect("Failed to write help text to buffer");
    let matches = app.get_matches();

    match matches.subcommand() {
        Some(("rebuild", sub)) => {
            pretty_env_logger::init();
            cli::rebuild(sub).await
        }
        Some(("deploy", sub)) => cli::deploy(sub).await,
        Some(("info", sub)) => {
            pretty_env_logger::init();
            cli::info(sub, cluster).await
        }
        Some(("ssh", sub)) => {
            pretty_env_logger::init();
            cli::ssh(sub).await
        }
        Some(("terraform", sub)) => {
            pretty_env_logger::init();
            cli::terraform(sub).await
        }
        Some(("provision", sub)) => {
            pretty_env_logger::init();
            cli::provision(sub).await
        }
        Some(("certs", sub)) => {
            pretty_env_logger::init();
            cli::certs(sub).await
        }
        _ => bail!(format!(
            "Invalid subcommand\n {}",
            String::from_utf8(help_text).expect("help text contains invalid UTF8")
        )),
    }?;
    Ok(())
}
