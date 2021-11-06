use clap::Parser;
use deploy::cli::Opts as Deploy;

#[derive(Parser)]
pub enum SubCommands {
    Info(Info),
    Ssh(Ssh),
    Terraform(Terraform),
    Deploy(Deploy),
}

#[derive(Parser)]
#[clap(about = "Show information about instances and auto-scaling groups")]
pub struct Info {
    #[clap(short, long, about = "output as JSON")]
    json: bool,
}

#[derive(Parser)]
#[clap(about = "SSH to instances")]
pub struct Ssh {
    #[clap(
        short,
        long,
        about = concat!(
            "specify client by: job, group, alloc_index\n",
            "additionally, this will auto 'cd' to alloc dir if <ARGS> are not specified"
        ),
        requires_all = &["nomad", "namespace"],
        number_of_values = 3,
        value_names = &["JOB", "GROUP", "INDEX"],
    )]
    job: Option<String>,
    #[clap(
        long,
        short,
        group = "multi",
        conflicts_with = "job",
        about = "run <ARGS> on all nodes",
        requires = "args"
    )]
    all: bool,
    #[clap(
        long,
        short,
        group = "multi",
        conflicts_with_all = &["all", "job"],
        about = "run <ARGS> on nodes in parallel",
        requires = "args"
    )]
    parallel: bool,
    #[clap(
        long,
        short,
        about = "for '-j': specify nomad namespace to search for <JOB>",
        env = "NOMAD_NAMESPACE"
    )]
    namespace: Option<String>,
    #[clap(
        long,
        short = 'l',
        about = "for '-a' or '-p': execute commands only on Nomad clients",
        requires = "multi"
    )]
    clients: bool,
    #[clap(
        long,
        short,
        about = "for '-a': seconds to delay between commands",
        requires = "all"
    )]
    delay: Option<usize>,
    #[clap(multiple_values = true, about = "arguments to ssh")]
    args: Option<String>,
}

#[derive(Parser)]
#[clap(about = "Run terraform", alias = "tf")]
pub struct Terraform {
    #[clap(about = "name of the terraform workspace")]
    workspace: String,
    #[clap(subcommand)]
    commands: TerraSubs,
}

#[derive(Parser)]
pub enum TerraSubs {
    Plan(Plan),
    Passthrough(Passthrough),
    Init(Init),
    #[clap(about = "terraform apply")]
    Apply,
}

#[derive(Parser)]
#[clap(about = "terraform plan")]
pub struct Plan {
    #[clap(long, short, about = "create a destruction plan")]
    destroy: bool,
}

#[derive(Parser)]
#[clap(about = "delegate to terraform", aliases = &["passthru", "pt"])]
pub struct Passthrough {
    #[clap(long, short, about = "skip regenerating the terraform config")]
    no_config: bool,
    #[clap(
        long,
        short,
        about = "delete and reinitialize the `.terraform` state dir before delegating to terraform"
    )]
    init: bool,
    #[clap(multiple_values = true, about = "arguments to terraform")]
    args: String,
}

#[derive(Parser)]
#[clap(about = "terraform init")]
pub struct Init {
    #[clap(long, short, about = "upgrade provider versions")]
    upgrade: bool,
}
