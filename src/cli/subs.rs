use anyhow::{Context, Result};
use clap::{AppSettings, ArgSettings, Parser};
use deploy::data as deployData;
use deploy::settings as deploySettings;
use uuid::Uuid;

#[derive(Parser)]
pub enum SubCommands {
    Info(Info),
    Ssh(Ssh),
    Deploy(Deploy),
    #[clap(setting = AppSettings::Hidden)]
    Completions(Completions),
}

#[derive(Parser)]
#[clap(about = "Show information about instances and auto-scaling groups")]
pub struct Info {
    #[clap(short, long, about = "output as JSON")]
    json: bool,
}

#[derive(Parser, Default)]
#[clap(about = "Deploy core and client nodes")]
pub struct Deploy {
    #[clap(long, short = 'l', about = "(re-)deploy all client nodes")]
    pub clients: bool,
    #[clap(flatten)]
    pub flags: deployData::Flags,

    #[clap(flatten)]
    pub generic_settings: deploySettings::GenericSettings,
    #[clap(
        about = concat!(
            "nodes to deploy; takes one or more needles to match against:\n",
            "private & public ip, node name and aws client id"
        ),
    )]
    pub nodes: Vec<String>,
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
        value_name = "TOKEN",
        about = "for '-j': The Nomad token used to query node information",
        env = "NOMAD_TOKEN",
        parse(try_from_str = token_context),
        setting = ArgSettings::HideEnvValues
    )]
    nomad: Option<Uuid>,
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
    #[clap(about = "arguments to ssh")]
    args: Option<String>,
}

#[derive(Parser)]
#[clap(about = "Generate CLI completions", alias = "comp")]
pub struct Completions {
    #[clap(subcommand)]
    shells: Shells,
}

#[derive(Parser)]
pub enum Shells {
    Bash,
    Zsh,
    Fish,
}

fn token_context(string: &str) -> Result<Uuid> {
    Uuid::parse_str(string).with_context(|| format!("'{}' is not a valid UUID", string))
}
