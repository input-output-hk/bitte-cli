use super::subs::SubCommands;
use crate::types::BitteProvider;
use anyhow::{Context, Result};
use clap::Parser;
use rusoto_core::Region;
use uuid::Uuid;

#[derive(Parser)]
pub struct Bitte {
    #[clap(
        arg_enum,
        long,
        about = "The cluster infrastructure provider",
        env = "BITTE_PROVIDER",
        case_insensitive = true
    )]
    provider: BitteProvider,
    #[clap(
        long,
        about = "The public domain of the cluster",
        env = "BITTE_DOMAIN",
        value_name = "NAME"
    )]
    domain: String,
    #[clap(
        long = "cluster",
        about = "The unique name of the cluster",
        env = "BITTE_CLUSTER",
        value_name = "TITLE"
    )]
    name: String,
    #[clap(
        long,
        global = true,
        value_name = "TOKEN",
        about = "The Nomad token used to query node information",
        env = "NOMAD_TOKEN",
        hidden = true,
        parse(try_from_str = token_context)
    )]
    nomad: Option<Uuid>,
    #[clap(
        long,
        about = "The default AWS region",
        env = "AWS_DEFAULT_REGION",
        value_name = "REGION",
        required_if_eq("provider", "AWS")
    )]
    aws_region: Option<Region>,
    #[clap(
        long,
        about = "Regions containing Nomad clients",
        env = "AWS_ASG_REGIONS",
        value_name = "REGIONS",
        required_if_eq("provider", "AWS"),
        value_delimiter(':'),
        require_delimiter = true
    )]
    aws_asg_regions: Option<Vec<Region>>,
    #[clap(subcommand)]
    commands: SubCommands,
}

fn token_context(string: &str) -> Result<Uuid> {
    Uuid::parse_str(string).with_context(|| format!("'{}' is not a valid UUID", string))
}
