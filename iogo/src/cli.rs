use anyhow::{Context, Result};
use bitte_lib::nomad::nomad_token;
use clap::ArgMatches;
use std::{env, process::Command};

pub(crate) async fn run(sub: &ArgMatches) -> Result<()> {
    let namespace: String = sub.value_of_t_or_exit("namespace");
    env::set_var("NOMAD_NAMESPACE", &namespace);
    Ok(())
}

pub(crate) async fn plan(sub: &ArgMatches) -> Result<()> {
    let namespace: String = sub.value_of_t_or_exit("namespace");
    let job: String = sub.value_of_t_or_exit("job");
    let vault_token: String = vault_print_token().or_else(vault_login)?;

    println!("job: {}, vault_token: {}", job, vault_token);

    env::set_var("NOMAD_NAMESPACE", &namespace);

    let token = nomad_token()?;
    println!("nomad token: {}", token);

    Ok(())
}

/*
status pending VAULT_TOKEN
VAULT_TOKEN="${VAULT_TOKEN:-}"

if ! vault token lookup &> /dev/null; then
    VAULT_TOKEN="$(vault login -method github -path github-employees -token-only)"
else
    VAULT_TOKEN="$(vault print token)"
fi

export VAULT_TOKEN
status ok VAULT_TOKEN
*/

fn vault_print_token() -> Result<String, anyhow::Error> {
    let mut cmd = Command::new("vault");
    let full = cmd.args(vec!["print", "token"]);
    let output = full.output().context("vault print token failed")?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

// TODO: give option to login using aws?
fn vault_login(_: anyhow::Error) -> Result<String, anyhow::Error> {
    let mut cmd = Command::new("vault");
    let full = cmd.args(vec![
        "login",
        "-method",
        "github",
        "-path",
        "github-employees",
        "-token-only",
    ]);

    let output = full.output().context("vault login failed")?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/*
NOMAD_TOKEN="${NOMAD_TOKEN:-}"
status pending NOMAD_TOKEN

if [ -z "$NOMAD_TOKEN" ] \
    || ! nomad acl token self | grep -v  'Secret ID' &> /dev/null; then

    NOMAD_TOKEN="$(vault read -field secret_id nomad/creds/developer)"
fi

export NOMAD_TOKEN
status ok NOMAD_TOKEN
*/
