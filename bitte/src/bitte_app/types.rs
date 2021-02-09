use std::collections::HashMap;

use restson::RestPath;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub(crate) struct TerraformCredentialFile {
    pub(crate) credentials: HashMap<String, TerraformCredential>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct TerraformCredential {
    pub(crate) token: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct HttpWorkspaces {
    pub(crate) data: Vec<HttpWorkspaceData>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct HttpWorkspace {
    pub(crate) data: HttpWorkspaceData,
    #[serde(rename = "type")]
    pub(crate) workspace_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct HttpPostWorkspaces {
    pub(crate) data: HttpPostWorkspaceData,
    #[serde(rename = "type")]
    pub(crate) workspace_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct HttpPostWorkspaceData {
    pub(crate) attributes: HttpWorkspaceDataAttributes,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct HttpWorkspaceData {
    pub(crate) id: String,
    pub(crate) attributes: HttpWorkspaceDataAttributes,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct HttpWorkspaceDataAttributes {
    pub(crate) name: String,
    pub(crate) operations: bool,
}

#[derive(Deserialize)]
pub(crate) struct HttpWorkspaceCurrentStateVersion {
    pub(crate) data: HttpWorkspaceCurrentStateVersionData,
}

#[derive(Deserialize)]
pub(crate) struct HttpWorkspaceCurrentStateVersionData {
    pub(crate) relationships: HttpWorkspaceCurrentStateVersionRelationships,
}

#[derive(Deserialize)]
pub(crate) struct HttpWorkspaceCurrentStateVersionRelationships {
    pub(crate) outputs: HttpWorkspaceCurrentStateVersionOutputs,
}

#[derive(Deserialize)]
pub(crate) struct HttpWorkspaceCurrentStateVersionOutputs {
    pub(crate) data: Vec<HttpWorkspaceCurrentStateVersionOutput>,
}

#[derive(Deserialize)]
pub(crate) struct HttpWorkspaceCurrentStateVersionOutput {
    pub(crate) id: String,
}

#[derive(Deserialize)]
pub(crate) struct HttpWorkspaceState {
    pub(crate) data: HttpWorkspaceStateData,
}

#[derive(Deserialize)]
pub(crate) struct HttpWorkspaceStateData {
    pub(crate) attributes: HttpWorkspaceStateAttributes,
}

#[derive(Deserialize)]
pub(crate) struct HttpWorkspaceStateAttributes {
    pub(crate) value: HttpWorkspaceStateValue,
}

#[derive(Deserialize)]
pub(crate) struct HttpWorkspaceStateValue {
    pub(crate) asgs: Option<HashMap<String, HttpWorkspaceStateAsg>>,
    pub(crate) instances: HashMap<String, HttpWorkspaceStateInstance>,
    #[serde(rename = "s3-cache")]
    pub(crate) s3_cache: String,
}

#[derive(Deserialize)]
pub(crate) struct HttpWorkspaceStateAsg {
    pub(crate) arn: String,
    pub(crate) region: String,
}

#[derive(Deserialize)]
pub(crate) struct HttpWorkspaceStateInstance {
    #[serde(rename = "flake-attr")]
    pub(crate) flake_attr: String,
    #[serde(rename = "instance-type")]
    pub(crate) instance_type: String,
    pub(crate) name: String,
    #[serde(rename = "private-ip")]
    pub(crate) private_ip: String,
    #[serde(rename = "public-ip")]
    pub(crate) public_ip: String,
    pub(crate) uid: String,
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
