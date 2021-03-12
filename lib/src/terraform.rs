use std::process::Command;
use std::{env, path::Path};
use std::{
    fs::{read_to_string, remove_dir_all},
    io::Read,
};

use anyhow::{anyhow, bail, Context, Result};
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
                .or_else(|_| bail!("Couldn't generate terraform config"))
        })?;
    Ok(())
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

fn terraform_vault_state(workspace: &str) -> Result<String> {
    let mut client = terraform_vault_client()?;
    let result: Result<RawVaultState, restson::Error> =
        client.get((bitte_cluster()?.as_str(), workspace));
    match result {
        Ok(value) => Ok(value.data.data.value),
        Err(e) => Err(e.into()),
    }
}

pub fn output(workspace: &str) -> Result<TerraformStateValue> {
    set_http_auth()?;
    let state = terraform_vault_state(workspace).context("failed to fetch state from vault")?;
    let decoded = base64::decode(state).context("failed to decode state")?;
    let mut decoder = ZlibDecoder::new(decoded.as_slice());
    let mut buf = "".to_string();
    decoder
        .read_to_string(&mut buf)
        .context("failed to inflate state")?;
    let state: TerraformState =
        serde_json::from_str(&buf).context("failed to decode state JSON")?;
    Ok(state.outputs.cluster.value)
}

fn github_token() -> Result<String> {
    let exp = &tilde("~/.netrc").to_string();
    let path = Path::new(exp);
    let netrc_file = read_to_string(path)?;
    let netrc = Netrc::parse(netrc_file, true).expect("invalid line in ~/.netrc");
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

    Err(anyhow!(
        "No entry for github.com or api.github.com found in ~/.netrc"
    ))
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
