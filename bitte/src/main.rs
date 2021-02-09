mod bitte_app;

use clap::clap_app;

#[tokio::main]
async fn main() {
    let matches = clap_app!(bitte =>
      (version: "0.0.1")
      (author: "Michael Fellinger <michael.fellinger@iohk.io>")
      (about: "Deploy all the things!")
      (@subcommand rebuild =>
        (about: "nixos-rebuild")
        (@arg only: --only +takes_value +multiple "pattern of hosts to deploy")
        (@arg delay: --delay +takes_value "seconds to delay between rebuilds"))
      (@subcommand info =>
        (about: "Show information about instances and auto-scaling groups"))
      (@subcommand ssh =>
        (about: "SSH to instances")
        (@arg host: +takes_value "host")
        (@arg args: +takes_value +multiple "arguments to ssh"))
      (@subcommand tf =>
        (about: "Run terraform")
        (@arg workspace: +takes_value +required "name of the terraform workspace")
        (@subcommand plan => (about: "terraform plan")
          (@arg destroy: "create a destruction plan"))
        (@subcommand apply => (about: "terraform apply"))
        (@subcommand workspaces => (about: "terraform workspaces list"))
       )
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
    .get_matches();

    match matches.subcommand() {
        Some(("rebuild", sub)) => bitte_app::cli_rebuild(sub).await,
        Some(("info", sub)) => bitte_app::cli_info(sub).await,
        Some(("ssh", sub)) => bitte_app::cli_ssh(sub).await,
        Some(("tf", sub)) => bitte_app::cli_tf(sub).await,
        Some(("provision", sub)) => bitte_app::cli_provision(sub).await,
        Some(("certs", sub)) => bitte_app::cli_certs(sub).await,
        _ => println!("Unknown command"),
    };
}
