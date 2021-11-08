use std::fs::{read_to_string, remove_dir_all};
use std::process::Command;
use std::{env, path::Path};

use crate::types::{error::Error, ClusterHandle};
use anyhow::{Context, Result};
use log::info;
use netrc_rs::Netrc;
use reqwest::Client;
use shellexpand::tilde;

use crate::types::{HttpPutToken, VaultLogin};

pub async fn prepare(workspace: String, cluster: ClusterHandle) -> Result<()> {
    set_http_auth().await?;
    info!("prepare terraform");
    generate_terraform_config(&workspace, cluster).await?;
    init(false).await?;
    Ok(())
}

pub async fn generate_terraform_config(workspace: &str, cluster: ClusterHandle) -> Result<()> {
    let cluster: String = cluster.await??.name;

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
        Err(Error::FailedTerraformConfig.into())
    }
}

pub async fn init(upgrade: bool) -> Result<()> {
    set_http_auth().await?;
    println!("run: terraform init");

    remove_dir_all(".terraform").ok();

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

fn github_token() -> Result<String> {
    let exp = &tilde("~/.netrc").to_string();
    let path = Path::new(exp);
    let netrc_file = read_to_string(path).map_err(|_| Error::NetrcMissing)?;
    let netrc = Netrc::parse(netrc_file, true).map_err(anyhow::Error::msg)?;
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

    Err(Error::NoGithubToken.into())
}

async fn vault_token() -> Result<String> {
    let gh_token = github_token()?;
    let client = Client::builder().gzip(true).build()?;
    let data = HttpPutToken { token: gh_token };
    let url = "https://vault.infra.aws.iohkdev.io/v1/auth/github-terraform/login";
    let result: VaultLogin = client
        .put(url)
        .json(&data)
        .send()
        .await
        .with_context(|| format!("failed to query: {}", &url))?
        .json::<VaultLogin>()
        .await
        .with_context(|| format!("failed to decode response from: {}", &url))?;
    Ok(result.auth.client_token)
}

pub async fn set_http_auth() -> Result<()> {
    if env::var("TF_HTTP_PASSWORD").is_ok() {
        info!("reusing existing TF_HTTP_* variables");
    } else {
        info!("set TF_HTTP_* variables");
        env::set_var("TF_HTTP_USERNAME", "TOKEN");
        env::set_var("TF_HTTP_PASSWORD", vault_token().await?);
    }

    Ok(())
}
