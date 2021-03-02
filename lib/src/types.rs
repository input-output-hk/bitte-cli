use std::collections::HashMap;

use restson::RestPath;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct TerraformCredentialFile {
    pub credentials: HashMap<String, TerraformCredential>,
}

#[derive(Deserialize, Debug)]
pub struct TerraformCredential {
    pub token: String,
}

#[derive(Deserialize, Debug)]
pub struct HttpWorkspaces {
    pub data: Vec<HttpWorkspaceData>,
}

#[derive(Deserialize, Debug)]
pub struct HttpWorkspace {
    pub data: HttpWorkspaceData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HttpPostWorkspaces {
    pub data: HttpPostWorkspaceData,
    #[serde(rename = "type")]
    pub workspace_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HttpPostWorkspaceData {
    pub attributes: HttpWorkspaceDataAttributes,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HttpWorkspaceData {
    pub id: String,
    pub attributes: HttpWorkspaceDataAttributes,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HttpWorkspaceDataAttributes {
    pub name: String,
    pub operations: bool,
}

#[derive(Deserialize)]
pub struct HttpWorkspaceCurrentStateVersion {
    pub data: HttpWorkspaceCurrentStateVersionData,
}

#[derive(Deserialize)]
pub struct HttpWorkspaceCurrentStateVersionData {
    pub relationships: HttpWorkspaceCurrentStateVersionRelationships,
}

#[derive(Deserialize)]
pub struct HttpWorkspaceCurrentStateVersionRelationships {
    pub outputs: HttpWorkspaceCurrentStateVersionOutputs,
}

#[derive(Deserialize)]
pub struct HttpWorkspaceCurrentStateVersionOutputs {
    pub data: Vec<HttpWorkspaceCurrentStateVersionOutput>,
}

#[derive(Deserialize)]
pub struct HttpWorkspaceCurrentStateVersionOutput {
    pub id: String,
}

#[derive(Deserialize)]
pub struct HttpWorkspaceState {
    pub data: HttpWorkspaceStateData,
}

#[derive(Deserialize)]
pub struct HttpWorkspaceStateData {
    pub attributes: HttpWorkspaceStateAttributes,
}

#[derive(Deserialize)]
pub struct HttpWorkspaceStateAttributes {
    pub value: HttpWorkspaceStateValue,
}

#[derive(Deserialize)]
pub struct HttpWorkspaceStateValue {
    pub asgs: Option<HashMap<String, HttpWorkspaceStateAsg>>,
    pub instances: HashMap<String, HttpWorkspaceStateInstance>,
    #[serde(rename = "s3-cache")]
    pub s3_cache: String,
}

#[derive(Deserialize)]
pub struct HttpWorkspaceStateAsg {
    pub arn: String,
    pub region: String,
    #[serde(rename = "flake-attr")]
    pub flake_attr: String,
    pub uid: String,
}

#[derive(Deserialize)]
pub struct HttpWorkspaceStateInstance {
    #[serde(rename = "flake-attr")]
    pub flake_attr: String,
    #[serde(rename = "instance-type")]
    pub instance_type: String,
    pub name: String,
    #[serde(rename = "private-ip")]
    pub private_ip: String,
    #[serde(rename = "public-ip")]
    pub public_ip: String,
    pub uid: String,
}

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

impl RestPath<&str> for HttpWorkspaces {
    fn get_path(org: &str) -> Result<String, restson::Error> {
        Ok(format!("/api/v2/organizations/{}/workspaces", org))
    }
}

impl RestPath<&str> for HttpPostWorkspaces {
    fn get_path(org: &str) -> Result<String, restson::Error> {
        Ok(format!("/api/v2/organizations/{}/workspaces", org))
    }
}

impl RestPath<(&str, &str)> for HttpWorkspace {
    fn get_path(params: (&str, &str)) -> Result<String, restson::Error> {
        let (org, name) = params;
        Ok(format!("/api/v2/organizations/{}/workspaces/{}", org, name))
    }
}

impl RestPath<&str> for HttpWorkspaceCurrentStateVersion {
    fn get_path(id: &str) -> Result<String, restson::Error> {
        Ok(format!("/api/v2/workspaces/{}/current-state-version", id))
    }
}

impl RestPath<&str> for HttpWorkspaceState {
    fn get_path(id: &str) -> Result<String, restson::Error> {
        Ok(format!("/api/v2/state-version-outputs/{}", id))
    }
}

impl RestPath<(&str, &str)> for RawVaultState {
    fn get_path(params: (&str, &str)) -> Result<String, restson::Error> {
        let (cluster, workspace) = params;
        Ok(format!("/v1/secret/data/vbk/{}/{}", cluster, workspace))
    }
}

// TODO: this will replace HttpWorkspaceState when we migrated all state

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

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
pub struct TerraformStateRoles {
    pub client: TerraformStateClient,
    pub core: TerraformStateClient,
}

#[derive(Serialize, Deserialize)]
pub struct TerraformStateClient {
    pub arn: String,
}
