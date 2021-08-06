use std::process::Command;
use std::{env, path::Path};
use std::{
    fs::{read_to_string, remove_dir_all},
    io::Read,
};

use crate::{error::Error, Result};
use flate2::read::ZlibDecoder;
use log::info;
use netrc_rs::Netrc;
use restson::RestClient;
use shellexpand::tilde;

use crate::types::{HttpPutToken, TerraformState, TerraformStateValue, VaultLogin};

use super::{bitte_cluster, types::RawVaultState};

pub fn prepare(workspace: String) -> Result<()> {
    set_http_auth()?;
    info!("prepare terraform");
    generate_terraform_config(&workspace)?;
    init(false)?;
    Ok(())
}

#[test]
fn test_generate_terraform_config() {
    assert!(
        generate_terraform_config("lies").is_err(),
        "should have errored out"
    )
}

pub fn generate_terraform_config(workspace: &str) -> Result<()> {
    let cluster = bitte_cluster()?;

    // To work on Darwin, we need to pass the current system
    let status = Command::new("nix")
        .arg("-L")
        .arg("run")
        .arg(format!(".#clusters.{}.tf.{}.config", cluster, workspace))
        .status()
        .and_then(|status| {
            if !status.success() {
                Command::new("nix")
                    .arg("-L")
                    .arg("run")
                    .arg(format!(".#clusters.{}.tf.{}.config", cluster, workspace))
                    .status()
            } else {
                Ok(status)
            }
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(Error::FailedTerraformConfig)
    }
}

pub fn init(upgrade: bool) -> Result<()> {
    set_http_auth()?;
    println!("run: terraform init");

    match remove_dir_all(".terraform") {
        Ok(_) => {}
        Err(_) => {}
    }

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
    Ok(())
}

fn terraform_vault_client() -> Result<RestClient> {
    let mut client = RestClient::new("https://vault.infra.aws.iohkdev.io")?;
    let token = vault_token()?;
    client.set_header("X-Vault-Token", &token)?;
    client.set_header("X-Vault-Request", "true")?;
    Ok(client)
}

fn terraform_vault_state(workspace: &str) -> Result<String> {
    let mut client = terraform_vault_client()?;
    let value: RawVaultState = client.get((bitte_cluster()?.as_str(), workspace))?;
    Ok(value.data.data.value)
}

pub fn output(workspace: &str) -> Result<TerraformStateValue> {
    set_http_auth()?;
    let state = terraform_vault_state(workspace)?;
    let decoded = base64::decode(state)?;
    let mut decoder = ZlibDecoder::new(decoded.as_slice());
    let mut buf = "".to_string();
    decoder.read_to_string(&mut buf)?;
    let state: TerraformState = serde_json::from_str(&buf)?;
    Ok(state.outputs.cluster.value)
}

fn github_token() -> Result<String> {
    let exp = &tilde("~/.netrc").to_string();
    let path = Path::new(exp);
    let netrc_file = read_to_string(path).or_else(|_| Err(Error::NetrcMissing))?;
    let netrc = Netrc::parse(netrc_file, true)?;
    for machine in &netrc.machines {
        if let Some(name) = &machine.name {
            if let ("github.com", Some(token)) = (name.as_str(), machine.password.as_ref()) {
                return Ok(token.to_string());
            }
            if let ("api.github.com", Some(token)) = (name.as_str(), machine.password.as_ref()) {
                return Ok(token.to_string());
            }
        };
    }

    Err(Error::NoGithubToken)
}

fn vault_token() -> Result<String> {
    let gh_token = github_token()?;
    let mut client = RestClient::new("https://vault.infra.aws.iohkdev.io")?;
    let data = HttpPutToken { token: gh_token };
    let result: VaultLogin = client.put_capture((), &data)?;
    Ok(result.auth.client_token)
}

pub fn set_http_auth() -> Result<()> {
    if env::var("TF_HTTP_PASSWORD").is_ok() {
        info!("reusing existing TF_HTTP_* variables");
    } else {
        info!("set TF_HTTP_* variables");
        env::set_var("TF_HTTP_USERNAME", "TOKEN");
        env::set_var("TF_HTTP_PASSWORD", vault_token()?);
    }

    Ok(())
}
