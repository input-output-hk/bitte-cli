use std::collections::HashMap;

use clap::ArgMatches;
use colored::*;
use restson::RestPath;
use rusoto_core::Region;
use rusoto_ec2::{DescribeInstancesRequest, Ec2, Ec2Client, Filter, Instance, Tag};
use serde::{de::Deserializer, Deserialize, Serialize};
use std::collections::hash_set::HashSet;
use std::env;
use std::io::BufReader;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use enum_utils::FromStr;
use std::net::{IpAddr, Ipv4Addr};
use uuid::Uuid;

use tokio::task::JoinHandle;

use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};

use crate::{terraform, Error};

use regex::Regex;

#[derive(Deserialize)]
pub struct RawVaultState {
    pub data: RawVaultStateData,
}

#[derive(Deserialize)]
pub struct RawVaultStateData {
    pub data: RawVaultStateDataData,
}

#[derive(Deserialize)]
pub struct RawVaultStateDataData {
    pub value: String,
}

impl RestPath<(&str, &str)> for RawVaultState {
    fn get_path(params: (&str, &str)) -> Result<String, restson::Error> {
        let (cluster, workspace) = params;
        Ok(format!("/v1/secret/data/vbk/{}/{}", cluster, workspace))
    }
}

impl RestPath<&str> for CueRender {
    fn get_path(id: &str) -> Result<String, restson::Error> {
        Ok(format!("/v1/job/{}/plan", id))
    }
}

impl RestPath<()> for CueRender {
    fn get_path(_: ()) -> Result<String, restson::Error> {
        Ok("/v1/jobs".to_string())
    }
}

impl RestPath<&str> for NomadEvaluation {
    fn get_path(eval_id: &str) -> Result<String, restson::Error> {
        Ok(format!("/v1/evaluation/{}", eval_id))
    }
}

impl RestPath<&str> for NomadDeployment {
    fn get_path(deployment_id: &str) -> Result<String, restson::Error> {
        Ok(format!("/v1/deployment/{}", deployment_id))
    }
}

impl RestPath<()> for HttpPutToken {
    fn get_path(_: ()) -> Result<String, restson::Error> {
        Ok("/v1/auth/github-terraform/login".to_string())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NomadDeployment {
    #[serde(rename = "Status")]
    pub status: NomadDeploymentStatus,
    #[serde(rename = "StatusDescription")]
    pub status_description: Option<String>,
    #[serde(rename = "TaskGroups")]
    pub task_groups: HashMap<String, NomadDeploymentTaskGroup>,
}

impl NomadDeployment {
    pub fn display(self: &NomadDeployment) {
        for (name, group) in &self.task_groups {
            println!(
                "
{}
auto promote: {}, auto revert: {}, promoted: {}
desired total: {}
canaries desired/placed: {}/{:?}
allocs placed/healthy/unhealthy {}/{}/{}
progress deadline: {}
require progress by: {}",
                name,
                group.auto_promote,
                group.auto_revert,
                group.promoted,
                group.desired_total,
                group.desired_canaries,
                group.placed_canaries,
                group.healthy_allocs,
                group.placed_allocs,
                group.unhealthy_allocs,
                group.progress_deadline,
                group.require_progress_by,
            );
        }

        match &self.status_description {
            Some(description) => match self.status {
                NomadDeploymentStatus::Running => {
                    println!("{}", description.yellow());
                }
                NomadDeploymentStatus::Complete => {
                    println!("{}", description.green());
                }
                NomadDeploymentStatus::Successful => {
                    println!("{}", description.green());
                }
                NomadDeploymentStatus::Failed => {
                    println!("{}", description.red());
                }
                NomadDeploymentStatus::Cancelled => {
                    println!("{}", description.red());
                }
            },
            None => {}
        }
    }

    pub fn is_done(self: &NomadDeployment) -> bool {
        match self.status {
            NomadDeploymentStatus::Running => false,
            NomadDeploymentStatus::Complete => true,
            NomadDeploymentStatus::Successful => true,
            NomadDeploymentStatus::Failed => true,
            NomadDeploymentStatus::Cancelled => true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NomadDeploymentTaskGroup {
    #[serde(rename = "AutoPromote")]
    pub auto_promote: bool,
    #[serde(rename = "AutoRevert")]
    pub auto_revert: bool,
    #[serde(rename = "DesiredCanaries")]
    pub desired_canaries: i64,
    #[serde(rename = "DesiredTotal")]
    pub desired_total: i64,
    #[serde(rename = "HealthyAllocs")]
    pub healthy_allocs: i64,
    #[serde(rename = "PlacedAllocs")]
    pub placed_allocs: i64,
    #[serde(rename = "PlacedCanaries")]
    pub placed_canaries: Option<Vec<String>>,
    #[serde(rename = "ProgressDeadline")]
    pub progress_deadline: i64,
    #[serde(rename = "Promoted")]
    pub promoted: bool,
    #[serde(rename = "RequireProgressBy")]
    pub require_progress_by: String,
    #[serde(rename = "UnhealthyAllocs")]
    pub unhealthy_allocs: i64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum NomadDeploymentStatus {
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "complete")]
    Complete,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "successful")]
    Successful,
    #[serde(rename = "cancelled")]
    Cancelled,
}

#[derive(Debug, Deserialize)]
pub struct NomadEvaluation {
    #[serde(rename = "CreateIndex")]
    pub create_index: i64,
    #[serde(rename = "CreateTime")]
    pub create_time: Option<f64>,
    #[serde(rename = "DeploymentID")]
    pub deployment_id: Option<String>,
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "JobID")]
    pub job_id: String,
    #[serde(rename = "JobModifyIndex")]
    pub job_modify_index: i64,
    #[serde(rename = "ModifyIndex")]
    pub modify_index: i64,
    #[serde(rename = "ModifyTime")]
    pub modify_time: Option<f64>,
    #[serde(rename = "Namespace")]
    pub namespace: Option<String>,
    #[serde(rename = "Priority")]
    pub priority: i64,
    #[serde(rename = "QueuedAllocations")]
    pub queued_allocations: Option<HashMap<String, i64>>,
    #[serde(rename = "SnapshotIndex")]
    pub snapshot_index: Option<i64>,
    #[serde(rename = "Status")]
    pub status: String,
    #[serde(rename = "TriggeredBy")]
    pub triggered_by: String,
    #[serde(rename = "Type")]
    pub nomad_evaluation_type: String,
    #[serde(rename = "NodeID")]
    pub node_id: Option<String>,
    #[serde(rename = "NodeModifyIndex")]
    pub node_modify_index: Option<i64>,
    #[serde(rename = "StatusDescription")]
    pub status_description: Option<String>,
    #[serde(rename = "Wait")]
    pub wait: Option<i64>,
    #[serde(rename = "NextEval")]
    pub next_eval: Option<String>,
    #[serde(rename = "PreviousEval")]
    pub previous_eval: Option<String>,
    #[serde(rename = "BlockedEval")]
    pub blocked_eval: Option<String>,
    #[serde(rename = "FailedTGAllocs")]
    pub failed_tg_allocs: Option<serde_json::Value>,
    #[serde(rename = "ClassEligibility")]
    pub class_eligibility: Option<serde_json::Value>,
    #[serde(rename = "EscapedComputedClass")]
    pub escaped_computed_class: Option<bool>,
    #[serde(rename = "AnnotatePlan")]
    pub annotate_plan: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct NomadJobRun {
    #[serde(rename = "EvalCreateIndex")]
    pub eval_create_index: i64,
    #[serde(rename = "EvalID")]
    pub eval_id: String,
    #[serde(rename = "Index")]
    pub index: i64,
    #[serde(rename = "JobModifyIndex")]
    pub job_modify_index: i64,
    #[serde(rename = "KnownLeader")]
    pub known_leader: bool,
    #[serde(rename = "LastContact")]
    pub last_contact: i64,
    #[serde(rename = "Warnings")]
    pub warnings: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NomadJobPlan {
    #[serde(rename = "Annotations")]
    pub annotations: NomadJobPlanAnnotations,
    #[serde(rename = "CreatedEvals")]
    pub created_evals: Option<serde_json::Value>,
    #[serde(rename = "Diff")]
    pub diff: NomadJobPlanDiff,
    #[serde(rename = "FailedTGAllocs")]
    pub failed_tg_allocs: Option<serde_json::Value>,
    #[serde(rename = "Index")]
    pub index: i64,
    #[serde(rename = "JobModifyIndex")]
    pub job_modify_index: i64,
    #[serde(rename = "NextPeriodicLaunch")]
    pub next_periodic_launch: Option<serde_json::Value>,
    #[serde(rename = "Warnings")]
    pub warnings: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NomadJobPlanAnnotations {
    #[serde(rename = "DesiredTGUpdates")]
    pub desired_tg_updates: HashMap<String, NomadJobPlanDesiredTgUpdate>,
    #[serde(rename = "PreemptedAllocs")]
    pub preempted_allocs: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NomadJobPlanDesiredTgUpdate {
    #[serde(rename = "Canary")]
    pub canary: i64,
    #[serde(rename = "DestructiveUpdate")]
    pub destructive_update: i64,
    #[serde(rename = "Ignore")]
    pub ignore: i64,
    #[serde(rename = "InPlaceUpdate")]
    pub in_place_update: i64,
    #[serde(rename = "Migrate")]
    pub migrate: i64,
    #[serde(rename = "Place")]
    pub place: i64,
    #[serde(rename = "Preemptions")]
    pub preemptions: i64,
    #[serde(rename = "Stop")]
    pub stop: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NomadJobPlanDiff {
    #[serde(rename = "Fields")]
    pub fields: Option<Vec<NomadJobPlanField>>,
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Objects")]
    pub objects: Option<Vec<NomadJobPlanObject>>,
    #[serde(rename = "TaskGroups")]
    pub task_groups: Vec<NomadJobPlanTaskGroup>,
    #[serde(rename = "Type")]
    pub diff_type: NomadJobPlanType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NomadJobPlanField {
    #[serde(rename = "Annotations")]
    pub annotations: Option<Vec<String>>,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "New")]
    pub new: String,
    #[serde(rename = "Old")]
    pub old: String,
    #[serde(rename = "Type")]
    pub field_type: NomadJobPlanType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NomadJobPlanTaskGroup {
    #[serde(rename = "Fields")]
    pub fields: Option<Vec<NomadJobPlanField>>,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Objects")]
    pub objects: Option<Vec<NomadJobPlanObject>>,
    #[serde(rename = "Tasks")]
    pub tasks: Option<Vec<NomadJobPlanObject>>,
    #[serde(rename = "Type")]
    pub task_group_type: NomadJobPlanType,
    #[serde(rename = "Updates")]
    pub updates: Option<NomadJobPlanUpdates>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NomadJobPlanObject {
    #[serde(rename = "Fields")]
    pub fields: Option<Vec<NomadJobPlanField>>,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Objects")]
    pub objects: Option<Vec<NomadJobPlanObject>>,
    #[serde(rename = "Type")]
    pub object_type: NomadJobPlanType,
    #[serde(rename = "Annotations")]
    pub annotations: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NomadJobPlanUpdates {
    pub create: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum NomadJobPlanType {
    Added,
    Deleted,
    Edited,
    None,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CueRender {
    #[serde(rename = "Job")]
    pub job: Job,
    #[serde(rename = "Diff")]
    pub diff: Option<bool>,
    #[serde(rename = "EnforceIndex")]
    pub enforce_index: Option<bool>,
    #[serde(rename = "JobModifyIndex")]
    pub job_modify_index: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Job {
    #[serde(rename = "Namespace")]
    pub namespace: String,
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Type")]
    pub job_type: String,
    #[serde(rename = "Priority")]
    pub priority: i64,
    #[serde(rename = "Periodic")]
    pub periodic: Option<Periodic>,
    #[serde(rename = "Datacenters")]
    pub datacenters: Vec<String>,
    #[serde(rename = "TaskGroups")]
    pub task_groups: Vec<Option<serde_json::Value>>,
    #[serde(rename = "Affinities")]
    pub affinities: Option<Vec<Option<serde_json::Value>>>,
    #[serde(rename = "Constraints")]
    pub constraints: Option<Vec<Option<serde_json::Value>>>,
    #[serde(rename = "Spreads")]
    pub spreads: Option<Vec<Option<serde_json::Value>>>,
    #[serde(rename = "ConsulToken")]
    pub consul_token: Option<String>,
    #[serde(rename = "VaultToken")]
    pub vault_token: Option<serde_json::Value>,
    #[serde(rename = "Vault")]
    pub vault: Option<serde_json::Value>,
    #[serde(rename = "Update")]
    pub update: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Periodic {
    #[serde(rename = "Enabled")]
    pub enabled: bool,
    #[serde(rename = "TimeZone")]
    pub time_zone: String,
    #[serde(rename = "SpecType")]
    pub spec_type: String,
    #[serde(rename = "Spec")]
    pub spec: String,
    #[serde(rename = "ProhibitOverlap")]
    pub prohibit_overlap: bool,
}

#[derive(Deserialize)]
pub struct ConsulAclTokenRead {
    #[serde(rename = "SecretID")]
    pub secret_id: String,
}

#[derive(Deserialize)]
pub struct VaultTokenLookup {
    pub data: VaultTokenLookupData,
}

#[derive(Deserialize)]
pub struct VaultTokenLookupData {
    pub id: String,
}

#[derive(Serialize)]
pub struct HttpPutToken {
    pub token: String,
}

#[derive(Serialize, Deserialize)]
pub struct VaultLogin {
    pub request_id: String,
    pub lease_id: String,
    pub renewable: bool,
    pub lease_duration: i64,
    pub auth: Auth,
}

#[derive(Serialize, Deserialize)]
pub struct Auth {
    pub client_token: String,
    pub accessor: String,
    pub policies: Vec<String>,
    pub token_policies: Vec<String>,
    pub metadata: Metadata,
    pub lease_duration: i64,
    pub renewable: bool,
    pub entity_id: String,
    pub token_type: String,
    pub orphan: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Metadata {
    pub org: String,
    pub username: String,
}

#[derive(Serialize, Deserialize)]
pub struct TerraformState {
    pub version: i64,
    pub terraform_version: String,
    pub serial: i64,
    pub lineage: String,
    pub outputs: TerraformStateOutputs,
}

#[derive(Serialize, Deserialize)]
pub struct TerraformStateOutputs {
    pub cluster: TerraformStateCluster,
}

#[derive(Serialize, Deserialize)]
pub struct TerraformStateCluster {
    pub value: TerraformStateValue,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TerraformStateValue {
    pub asgs: HashMap<String, TerraformStateAsg>,
    pub flake: String,
    pub instances: HashMap<String, TerraformStateInstance>,
    pub kms: String,
    pub name: String,
    pub nix: String,
    pub region: String,
    pub roles: TerraformStateRoles,
    #[serde(rename = "s3-bucket")]
    pub s3_bucket: String,
    #[serde(rename = "s3-cache")]
    pub s3_cache: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TerraformStateAsg {
    pub arn: String,
    pub count: i64,
    #[serde(rename = "flake-attr")]
    pub flake_attr: String,
    #[serde(rename = "instance-type")]
    pub instance_type: String,
    pub region: String,
    pub uid: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TerraformStateInstance {
    #[serde(rename = "flake-attr")]
    pub flake_attr: String,
    #[serde(rename = "instance-type")]
    pub instance_type: String,
    pub name: String,
    #[serde(rename = "private-ip")]
    pub private_ip: String,
    #[serde(rename = "public-ip")]
    pub public_ip: String,
    pub tags: HashMap<String, String>,
    pub uid: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TerraformStateRoles {
    pub client: TerraformStateClient,
    pub core: TerraformStateClient,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TerraformStateClient {
    pub arn: String,
}

/// A description of a Bitte cluster and its nodes
#[derive(Debug, Serialize, Deserialize)]
pub struct BitteCluster {
    pub name: String,
    pub nodes: BitteNodes,
    pub domain: String,
    pub provider: BitteProvider,
    #[serde(skip_serializing_if = "skip_info")]
    pub terra: Option<TerraformStateValue>,
    #[serde(skip)]
    pub nomad_api_client: Arc<Client>,
    pub ttl: SystemTime,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, FromStr)]
pub enum BitteProvider {
    #[allow(clippy::upper_case_acronyms)]
    AWS,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct NomadClient {
    #[serde(rename = "ID")]
    pub id: Uuid,
    pub allocs: Option<NomadAllocs>,
    #[serde(rename = "Address")]
    pub address: Option<IpAddr>,
}

impl NomadClient {
    async fn find_nomad_nodes(client: Arc<Client>, domain: String) -> Result<NomadClients> {
        let url = format!("https://nomad.{}/v1/nodes", domain);
        let nodes = client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("failed to query: {}", &url))?
            .json::<NomadClients>()
            .await
            .with_context(|| format!("failed to decode response from: {}", &url))?;
        Ok(nodes)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BitteNode {
    pub id: String,
    pub name: String,
    pub priv_ip: IpAddr,
    pub pub_ip: IpAddr,
    pub nixos: String,
    #[serde(skip_serializing_if = "skip_info")]
    pub nomad_client: Option<NomadClient>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asg: Option<String>,
}

fn skip_info<T>(_: &Option<T>) -> bool {
    env::var("BITTE_INFO_NO_ALLOCS").is_ok()
}

pub trait BitteFind
where
    Self: IntoIterator,
{
    fn find_needle(self, needle: &str) -> Result<Self::Item>;
    fn find_needles(self, needles: Vec<&str>) -> Self;
}

impl BitteFind for BitteNodes {
    fn find_needle(self, needle: &str) -> Result<Self::Item> {
        self.into_iter()
            .find(|node| {
                let ip = needle.parse::<IpAddr>().ok();

                node.id == needle
                    || node.name == needle
                    || node
                        .nomad_client
                        .as_ref()
                        .unwrap_or(&Default::default())
                        .id
                        .to_hyphenated()
                        .to_string()
                        == needle
                    || Some(node.priv_ip) == ip
                    || Some(node.pub_ip) == ip
            })
            .with_context(|| format!("{} does not match any nodes", needle))
    }

    fn find_needles(self, needles: Vec<&str>) -> Self {
        self.into_iter()
            .filter(|node| {
                let ips: Vec<Option<IpAddr>> = needles
                    .iter()
                    .map(|needle| needle.parse::<IpAddr>().ok())
                    .collect();

                needles.contains(&&*node.id)
                    || needles.contains(&&*node.name)
                    || needles.contains(
                        &&*node
                            .nomad_client
                            .as_ref()
                            .unwrap_or(&Default::default())
                            .id
                            .to_hyphenated()
                            .to_string(),
                    )
                    || ips.contains(&Some(node.priv_ip))
                    || ips.contains(&Some(node.pub_ip))
            })
            .collect()
    }
}

impl From<Instance> for BitteNode {
    fn from(instance: Instance) -> Self {
        let tags = instance.tags.unwrap_or_default();

        let nixos = tags
            .iter()
            .find(|tag| tag.key == Some("UID".into()))
            .unwrap_or(&Tag {
                key: None,
                value: None,
            })
            .value
            .as_ref();

        let name = tags
            .iter()
            .find(|tag| tag.key == Some("Name".into()))
            .unwrap_or(&Tag {
                key: None,
                value: None,
            })
            .value
            .as_ref();

        let asg = tags
            .iter()
            .find(|tag| tag.key == Some("aws:autoscaling:groupName".into()))
            .unwrap_or(&Tag {
                key: None,
                value: None,
            })
            .value
            .as_ref();

        let no_ip = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));

        let zone = if let Some(p) = instance.placement {
            p.availability_zone
        } else {
            None
        };

        Self {
            id: instance.instance_id.unwrap_or_default(),
            name: match name {
                Some(name) => name.to_owned(),
                None => "".into(),
            },
            priv_ip: IpAddr::from_str(&instance.private_ip_address.unwrap_or_default())
                .unwrap_or(no_ip),
            pub_ip: IpAddr::from_str(&instance.public_ip_address.unwrap_or_default())
                .unwrap_or(no_ip),
            nomad_client: None,
            nixos: match nixos {
                Some(nixos) => nixos.to_owned(),
                None => "".into(),
            },
            node_type: instance.instance_type,
            zone,
            asg: asg.map(|asg| asg.to_owned()),
        }
    }
}

impl BitteNode {
    async fn find_nodes(
        provider: BitteProvider,
        name: String,
        allocs: AllocHandle,
        clients: ClientHandle,
        state: TerraHandle,
        args: ArgMatches,
    ) -> Result<(BitteNodes, Option<TerraformStateValue>)> {
        match provider {
            BitteProvider::AWS => {
                let regions: HashSet<String> = {
                    let mut result = args.values_of_t("aws-asg-regions")?;
                    let default = args.value_of_t("aws-region")?;
                    result.push(default);
                    result.into_iter().collect()
                };

                let mut handles = Vec::new();

                for region_str in regions {
                    let region = Region::from_str(&region_str)?;
                    let client = Ec2Client::new(region);
                    let request = DescribeInstancesRequest {
                        instance_ids: None,
                        dry_run: None,
                        filters: Some(vec![
                            Filter {
                                name: Some("tag:Cluster".to_owned()),
                                values: Some(vec![name.to_owned()]),
                            },
                            Filter {
                                name: Some("instance-state-name".to_owned()),
                                values: Some(vec!["running".to_owned()]),
                            },
                        ]),
                        max_results: None,
                        next_token: None,
                    };
                    let response = tokio::spawn(async move {
                        client.describe_instances(request).await.with_context(|| {
                            format!("failed to connect to ec2.{}.amazonaws.com", region_str)
                        })
                    });
                    handles.push(response);
                }

                let mut result: BitteNodes = Vec::new();

                let allocs = allocs.await??;
                let clients = clients.await??;

                let state = if let Some(state) = state {
                    Some(state.await??)
                } else {
                    None
                };

                for response in handles.into_iter() {
                    let response = response.await??;
                    let iter = response.reservations.into_iter();
                    let mut nodes: BitteNodes = iter
                        .flat_map(|reservations| {
                            reservations
                                .into_iter()
                                .flat_map(|reservation| reservation.instances.unwrap_or_default())
                        })
                        .map(|instance| {
                            let mut node = BitteNode::from(instance);
                            node.nomad_client = match clients
                                .iter()
                                .find(|client| client.address == Some(node.priv_ip))
                            {
                                Some(client) => {
                                    let mut client = client.to_owned();
                                    client.allocs = {
                                        Some(
                                            allocs
                                                .iter()
                                                .filter(|alloc| alloc.node_id == client.id)
                                                .map(|alloc| alloc.to_owned())
                                                .collect::<NomadAllocs>(),
                                        )
                                    };
                                    Some(client)
                                }
                                None => None,
                            };

                            if node.name.is_empty() {
                                if let Some(state) = &state {
                                    for inst in state.instances.values() {
                                        if inst.private_ip == node.priv_ip.to_string() {
                                            node.name = inst.name.clone()
                                        };
                                    }
                                };
                            }

                            node
                        })
                        .collect();

                    result.append(&mut nodes);
                }

                Ok((result, state))
            }
        }
    }
}

type NomadClients = Vec<NomadClient>;
type NomadAllocs = Vec<NomadAlloc>;
type BitteNodes = Vec<BitteNode>;
pub type ClusterHandle = JoinHandle<Result<BitteCluster>>;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum AllocIndex {
    Int(u32),
    String(String),
}

impl AllocIndex {
    pub fn get(&self) -> Option<u32> {
        match self {
            Self::Int(i) => Some(*i),
            Self::String(_) => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NomadAlloc {
    #[serde(rename = "ID")]
    pub id: Uuid,
    #[serde(rename = "JobID")]
    pub job_id: String,
    #[serde(rename = "Namespace")]
    pub namespace: String,
    #[serde(rename = "TaskGroup")]
    pub task_group: String,
    #[serde(rename = "ClientStatus")]
    pub status: String,
    #[serde(
        rename(deserialize = "Name", serialize = "Index"),
        deserialize_with = "pull_index"
    )]
    #[serde(alias = "Index")]
    pub index: AllocIndex,
    #[serde(rename = "NodeID")]
    pub node_id: Uuid,
}

impl NomadAlloc {
    async fn find_allocs(client: Arc<Client>, domain: String) -> Result<NomadAllocs> {
        let url = format!("https://nomad.{}/v1/allocations", domain);
        let allocs = client
            .get(&url)
            .query(&[("namespace", "*"), ("task_states", "false")])
            .send()
            .await
            .with_context(|| format!("failed to query: {}", &url))?
            .json::<NomadAllocs>()
            .await
            .with_context(|| format!("failed to decode response from: {}", &url))?;
        Ok(allocs)
    }
}

fn pull_index<'de, D>(deserializer: D) -> Result<AllocIndex, D::Error>
where
    D: Deserializer<'de>,
{
    let buf = AllocIndex::deserialize(deserializer)?;

    match buf {
        AllocIndex::Int(i) => Ok(AllocIndex::Int(i)),
        AllocIndex::String(s) => {
            let search = Regex::new("[0-9]*\\]$").unwrap().find(&s).unwrap().as_str();

            let index = &search[0..search.len() - 1];
            let index: u32 = index.parse().unwrap();

            Ok(AllocIndex::Int(index))
        }
    }
}

type TerraHandle = Option<JoinHandle<Result<TerraformStateValue>>>;
type ClientHandle = JoinHandle<Result<NomadClients>>;
type AllocHandle = JoinHandle<Result<NomadAllocs>>;

impl BitteCluster {
    pub async fn new(args: &ArgMatches, token: Uuid) -> Result<Self> {
        let name: String = args.value_of_t("name")?;
        let domain: String = args.value_of_t("domain")?;
        let provider: BitteProvider = {
            let provider: String = args.value_of_t("provider")?;
            match provider.parse() {
                Ok(v) => Ok(v),
                Err(_) => Err(Error::ProviderError { provider }),
            }?
        };

        let t_state = match &provider {
            BitteProvider::AWS => Some(tokio::spawn(async move {
                terraform::output("clients")
                    .or_else::<anyhow::Error, _>(|_| terraform::output("core"))
            })),
        };

        let nomad_api_client = {
            let mut token = HeaderValue::from_str(&token.to_string())?;
            token.set_sensitive(true);
            let mut headers = HeaderMap::new();
            headers.insert("X-Nomad-Token", token);
            Arc::new(
                Client::builder()
                    .default_headers(headers)
                    .gzip(true)
                    .build()?,
            )
        };

        let allocs = tokio::spawn(NomadAlloc::find_allocs(
            Arc::clone(&nomad_api_client),
            domain.to_owned(),
        ));

        let client_nodes = tokio::spawn(NomadClient::find_nomad_nodes(
            Arc::clone(&nomad_api_client),
            domain.to_owned(),
        ));

        let args = args.clone();

        let nodes = tokio::spawn(BitteNode::find_nodes(
            provider,
            name.to_owned(),
            allocs,
            client_nodes,
            t_state,
            args,
        ));

        let (nodes, terra) = nodes.await??;

        let cluster = Self {
            name,
            domain,
            provider,
            nomad_api_client,
            nodes,
            terra,
            ttl: SystemTime::now()
                .checked_add(Duration::from_secs(300))
                .unwrap(),
        };

        let file = std::fs::File::create(cache_dir()?).ok();

        if let Some(file) = file {
            serde_json::to_writer(file, &cluster)?;
        }

        Ok(cluster)
    }

    #[inline(always)]
    pub fn init(args: ArgMatches, token: Uuid) -> ClusterHandle {
        tokio::spawn(async move {
            let file = std::fs::File::open(cache_dir()?).ok();

            let cluster: BitteCluster;

            if let Some(file) = file {
                let reader = BufReader::new(file);

                cluster = {
                    let cluster = {
                        let cluster = serde_json::from_reader(reader);
                        match cluster.ok() {
                            Some(c) => c,
                            None => BitteCluster::new(&args, token).await?,
                        }
                    };
                    match cluster.ttl.duration_since(SystemTime::now()) {
                        Ok(_) => cluster,
                        Err(_) => BitteCluster::new(&args, token).await?,
                    }
                }
            } else {
                cluster = BitteCluster::new(&args, token).await?;
            }

            Ok(cluster)
        })
    }
}

fn cache_dir() -> Result<String> {
    Ok(format!(
        "{}/bitte.json",
        env::var("XDG_CACHE_DIR")
            .or_else::<anyhow::Error, _>(|_| Ok(format!("{}/.cache", env::var("HOME")?)))?
    ))
}
