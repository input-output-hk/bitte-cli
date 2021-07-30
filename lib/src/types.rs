use std::collections::HashMap;

use colored::*;
use restson::RestPath;
use serde::{de::Deserializer, Deserialize, Serialize};
use std::sync::Arc;

use std::net::IpAddr;
use uuid::Uuid;

use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};

use regex::Regex;

use crate::nomad;

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
        Ok(format!("/v1/job/{}/plan", id).to_string())
    }
}

impl RestPath<()> for CueRender {
    fn get_path(_: ()) -> Result<String, restson::Error> {
        Ok("/v1/jobs".to_string())
    }
}

impl RestPath<&str> for NomadEvaluation {
    fn get_path(eval_id: &str) -> Result<String, restson::Error> {
        Ok(format!("/v1/evaluation/{}", eval_id).to_string())
    }
}

impl RestPath<&str> for NomadDeployment {
    fn get_path(deployment_id: &str) -> Result<String, restson::Error> {
        Ok(format!("/v1/deployment/{}", deployment_id).to_string())
    }
}

impl RestPath<()> for HttpPutToken {
    fn get_path(_: ()) -> Result<String, restson::Error> {
        Ok("/v1/auth/github-employees/login".to_string())
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
                    return;
                }
                NomadDeploymentStatus::Successful => {
                    println!("{}", description.green());
                    return;
                }
                NomadDeploymentStatus::Failed => {
                    println!("{}", description.red());
                    return;
                }
                NomadDeploymentStatus::Cancelled => {
                    println!("{}", description.red());
                    return;
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
    pub nodes: Vec<BitteNode>,
    pub allocs: Vec<NomadAlloc>,
    pub domain: String,
    pub provider: BitteProvider,
    #[serde(skip)]
    pub nomad_client: Arc<Client>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BitteProvider {
    AWS,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BitteNode {
    pub id: String,
    pub priv_ip: IpAddr,
    pub pub_ip: IpAddr,
    pub region: Option<String>,
    pub nixos: String,
    pub nomad_id: Option<Uuid>,
    /// store the indices of `BitteCluster.allocs` running on this node or `None` if not a Nomad client
    pub allocs: Option<Vec<usize>>,
}

#[derive(Debug, Serialize, Deserialize)]
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
    pub index: u32,
    #[serde(rename = "NodeID")]
    pub node_id: Uuid,
}

fn pull_index<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;
    let search = Regex::new("[0-9]*\\]$")
        .unwrap()
        .find(&buf)
        .unwrap()
        .as_str();

    let index = &search[0..search.len() - 1];
    let index: u32 = index.parse().unwrap();

    Ok(index)
}

impl BitteCluster {
    pub async fn new(
        name: String,
        domain: String,
        provider: BitteProvider,
    ) -> anyhow::Result<Self> {
        let nomad_client = {
            let mut token = HeaderValue::from_str(nomad::nomad_token()?.as_str())?;
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
        let allocs = tokio::spawn(BitteCluster::find_allocs(
            Arc::clone(&nomad_client),
            domain.clone(),
        ));
        let nodes = tokio::spawn(BitteCluster::find_nodes(provider.clone()));

        let allocs = allocs.await??;
        let nodes = nodes.await?;
        Ok(Self {
            name,
            domain,
            provider,
            nomad_client,
            nodes,
            allocs,
        })
    }

    async fn find_nodes(provider: BitteProvider) -> Vec<BitteNode> {
        match provider {
            BitteProvider::AWS => return Vec::new(),
        }
    }

    async fn find_allocs(client: Arc<Client>, domain: String) -> anyhow::Result<Vec<NomadAlloc>> {
        let allocs = client
            .get(format!("https://nomad.{}/v1/allocations", domain))
            .query(&[("namespace", "*"), ("task_states", "false")])
            .send()
            .await?
            .json::<Vec<NomadAlloc>>()
            .await?;
        Ok(allocs)
    }
}
