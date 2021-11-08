use super::subs::SubCommands;
use crate::types::BitteProvider;
use clap::Parser;
use rusoto_core::Region;

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
