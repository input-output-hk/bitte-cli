use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::{fs, process::Command};

use anyhow::{Context, Result};
use log::{debug, info};
use restson::RestClient;
use shellexpand::tilde;

use super::{
    bitte_cluster,
    types::{
        HttpPostWorkspaceData, HttpPostWorkspaces, HttpWorkspace, HttpWorkspaceCurrentStateVersion,
        HttpWorkspaceData, HttpWorkspaceDataAttributes, HttpWorkspaceState,
        HttpWorkspaceStateValue, HttpWorkspaces, RawVaultState, TerraformCredentialFile,
    },
};

pub fn workspace_list() -> Result<Vec<HttpWorkspaceData>> {
    let mut client = terraform_client()?;
    let workspaces: Result<HttpWorkspaces, restson::Error> =
        client.get(terraform_organization()?.as_str());
    match workspaces {
        Ok(list) => Ok(list.data),
        Err(e) => Err(e.into()),
    }
}

pub fn prepare(workspace: String) -> Result<()> {
    info!("prepare terraform");
    // To work on Darwin, we need to pass the current system

    generate_terraform_config(&workspace)?;

    let original = workspace_show();
    info!("original: {}, workspace: {}", original, workspace);
    if original != workspace {
        let list: Vec<String> = workspace_list()
            .unwrap_or_else(|_| vec![])
            .iter()
            .map(|w| w.attributes.name.clone())
            .collect();
        debug!("{:?}", list);
        let workspace_fullname = format!("{}_{}", bitte_cluster()?, workspace);
        if !list.contains(&workspace_fullname) {
            workspace_new(&workspace_fullname)
                .context(format!("Failed to create workspace {}", workspace))?;
        }
        workspace_select(workspace);
    }
    init(false);
    Ok(())
}

pub fn current_state_version(workspace_id: &str) -> Result<String> {
    let mut client = terraform_client()?;
    let current_state_version: Result<HttpWorkspaceCurrentStateVersion, restson::Error> =
        client.get(workspace_id);
    match current_state_version {
        Ok(version) => Ok(version.data.relationships.outputs.data[0].id.to_string()),
        Err(e) => Err(e.into()),
    }
}

pub fn generate_terraform_config(workspace: &str) -> Result<()> {
    let cluster = bitte_cluster()?;
    Command::new("nix")
        .arg("-L")
        .arg("run")
        .arg(format!(
            ".#clusters.{}.{}.tf.{}.config",
            nix_current_system(),
            cluster,
            workspace
        ))
        .status()
        .or_else(|_| {
            Command::new("nix")
                .arg("-L")
                .arg("run")
                .arg(format!(".#clusters.{}.tf.{}.config", cluster, workspace))
                .status()
        })?;
    Ok(())
}

fn workspace_new(workspace: &str) -> Result<()> {
    let mut client = terraform_client()?;
    let body = HttpPostWorkspaces {
        workspace_type: "workspace".to_string(),
        data: HttpPostWorkspaceData {
            attributes: HttpWorkspaceDataAttributes {
                name: workspace.to_string(),
                operations: false,
            },
        },
    };
    client.post_with(terraform_organization()?.as_str(), &body, &[])?;
    Ok(())
}

fn workspace_show() -> String {
    match fs::read(".terraform/environment") {
        Ok(content) => String::from_utf8_lossy(&content).to_string(),
        Err(e) => {
            debug!("error while reading workspace: {:?}", e);
            "default".to_string()
        }
    }
}

fn workspace_select(workspace: String) {
    println!("run: terraform workspace select {}", workspace);
    Command::new("terraform")
        .arg("workspace")
        .arg("select")
        .arg(workspace)
        .status()
        .expect("terraform workspace select failed");
}

pub fn init(upgrade: bool) {
    println!("run: terraform init");
    if upgrade {
        Command::new("terraform")
            .args(&["init", "-upgrade"])
            .status()
            .expect("terraform init -upgrade failed")
    } else {
        Command::new("terraform")
            .args(&["init"])
            .status()
            .expect("terraform init failed")
    };
}

fn nix_current_system() -> String {
    let result = Command::new("nix")
        .args(&[
            "eval",
            "--impure",
            "--raw",
            "--expr",
            "builtins.currentSystem",
        ])
        .output();
    match result {
        Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_string(),
        Err(_) => "x86_64-linux".into(),
    }
}

pub fn fetch_current_state_version(workspace_name_suffix: &str) -> Result<String> {
    let terraform_organization = terraform_organization()?;
    let workspace_name = format!("{}_{}", bitte_cluster()?, workspace_name_suffix);
    let workspace_id = workspace_id(terraform_organization.as_str(), workspace_name.as_str())?;
    let result = current_state_version(&workspace_id)?;
    Ok(result)
}

pub fn current_state_version_output(state_id: &str) -> Result<HttpWorkspaceStateValue> {
    let mut client = terraform_client()?;
    let current_state_version_output: Result<HttpWorkspaceState, restson::Error> =
        client.get(state_id);
    match current_state_version_output {
        Ok(output) => Ok(output.data.attributes.value),
        Err(e) => Err(e.into()),
    }
}

fn workspace_id(organization: &str, workspace: &str) -> Result<String> {
    let mut client = terraform_client()?;
    let params = (organization, workspace);
    let workspace: Result<HttpWorkspace, restson::Error> = client.get(params);
    match workspace {
        Ok(workspace) => Ok(workspace.data.id),
        Err(e) => Err(e.into()),
    }
}

fn terraform_client() -> Result<RestClient> {
    let mut client =
        RestClient::new("https://app.terraform.io").context("Couldn't create RestClient")?;
    let token = terraform_token()
        .context("Make sure you are logged into terraform: run `terraform login`")?;
    client
        .set_header("Authorization", format!("Bearer {}", token).as_str())
        .context("Coudln't set Authorization header")?;
    client
        .set_header("Content-Type", "application/vnd.api+json")
        .context("Couldn't set Content-Type header")?;
    Ok(client)
}

fn terraform_token() -> Result<String> {
    let creds = parse_terraform_credentials();
    let c = &creds?.credentials["app.terraform.io"];
    let token = &c.token;
    Ok(token.to_string())
}

fn parse_terraform_credentials() -> Result<TerraformCredentialFile> {
    let exp = &tilde("~/.terraform.d/credentials.tfrc.json").to_string();
    let path = Path::new(exp);
    let file = File::open(path).context(format!("Couldn't read {}", exp))?;
    let reader = BufReader::new(file);
    let creds: TerraformCredentialFile =
        serde_json::from_reader(reader)
        .context(format!("Couldn't parse {}", exp))?;
    Ok(creds)
}

fn terraform_organization() -> Result<String> {
    let org = env::var("TERRAFORM_ORGANIZATION")
        .context("TERRAFORM_ORGANIZATION environment variable must be set")?;
    Ok(org)
}

fn vault_token() -> Result<String> {
    println!("run: vault print token");
    let output = Command::new("vault")
        .args(vec!["print", "token"])
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn terraform_vault_client() -> Result<RestClient> {
    let mut client = RestClient::new("https://vault.infra.aws.iohkdev.io")
        .context("Couldn't create RestClient")?;
    let token = vault_token().context("Make sure you are logged into vault: run `vault login`")?;
    client
        .set_header("X-Vault-Token", &token)
        .context("Couldn't set X-Vault-Token header")?;
    client
        .set_header("X-Vault-Request", "true")
        .context("Couldn't set X-Vault-Request header")?;
    Ok(client)
}

fn terraform_vault_state(workspace: String) -> Result<String> {
    let mut client = terraform_vault_client()?;
    let result: Result<RawVaultState, restson::Error> =
        client.get((terraform_organization()?.as_str(), workspace.as_str()));
    match result {
        Ok(value) => Ok(value.data.data.value),
        Err(e) => Err(e.into()),
    }
}

use std::io::Read;
pub fn output(workspace: String) -> Result<()> {
    let state = terraform_vault_state(workspace)?;
    let decoded = base64::decode(state)?;
    let mut decoder = flate2::read::ZlibDecoder::new(decoded.as_slice());
    let mut buf = "".to_string();
    decoder.read_to_string(&mut buf)?;
    let state : crate::types::TerraformState = serde_json::from_str(&buf)?;
    println!("{}", state.outputs.cluster.value.flake);
    Ok(())
}
