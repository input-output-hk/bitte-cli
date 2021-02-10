mod info;
mod provision;
mod rebuild;
mod terraform;
mod types;
mod certs;
mod ssh;

use clap::ArgMatches;
use execute::Execute;
use restson::RestClient;
use shellexpand::tilde;
use tokio::{net::TcpStream, time::timeout};
use std::fs::File;
use std::process::Command;
use std::{env, error::Error};
use std::{fmt, path::Path, process::Stdio};
use std::{io::BufReader, time::Duration};

use self::{
    info::cli_info_print,
    rebuild::{rebuild_copy, set_ssh_opts},
    terraform::current_state_version,
    types::{HttpWorkspace, HttpWorkspaceState, HttpWorkspaceStateValue, TerraformCredentialFile},
};

pub(crate) async fn cli_certs(sub: &ArgMatches) {
    certs::cli_certs(sub).await
}

pub(crate) async fn cli_provision(sub: &ArgMatches) {
    provision::cli_provision(sub).await
}

pub(crate) async fn cli_ssh(sub: &ArgMatches) {
    ssh::cli_ssh(sub).await
}

pub(crate) async fn cli_info(_sub: &ArgMatches) {
    let info = fetch_current_state_version("clients")
        .or_else(|_| fetch_current_state_version("core"))
        .expect("Coudln't fetch clients or core workspaces");
    cli_info_print(info).await;
}

pub(crate) async fn cli_tf(sub: &ArgMatches) {
    let workspace: String = sub
        .value_of_t("workspace")
        .expect("workspace argument is missing");

    match sub.subcommand() {
        Some(("plan", sub_sub)) => terraform::cli_tf_plan(workspace, sub_sub).await,
        Some(("apply", sub_sub)) => terraform::cli_tf_apply(workspace, sub_sub).await,
        Some(("workspaces", sub_sub)) => terraform::cli_tf_workspaces(workspace, sub_sub).await,
        _ => println!("Unknown command"),
    }
}

pub(crate) async fn cli_rebuild(sub: &ArgMatches) {
    let only: Vec<String> = sub.values_of_t("only").unwrap_or(vec![]);
    let delay = Duration::from_secs(sub.value_of_t::<u64>("delay").unwrap_or(0));

    set_ssh_opts(true);
    rebuild_copy(&only, delay).await;
}

fn bitte_cluster() -> String {
    env::var("BITTE_CLUSTER").expect("BITTE_CLUSTER environment variable must be set")
}

fn handle_command_error(mut command: std::process::Command) -> Result<String, ExeError> {
    println!("running: {:?}", command);
    // command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    match command.execute_output() {
        Ok(output) => match output.status.code() {
            Some(exit_code) => {
                if exit_code == 0 {
                    Ok("Ok".to_string())
                } else {
                    Err(ExeError {
                        details: String::from_utf8_lossy(&output.stderr).to_string(),
                    })
                }
            }
            None => Err(ExeError {
                details: "interrupted".to_string(),
            }),
        },
        Err(e) => Err(ExeError {
            details: e.to_string(),
        }),
    }
}

#[derive(Debug)]
struct ExeError {
    details: String,
}

impl fmt::Display for ExeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for ExeError {
    fn description(&self) -> &str {
        &self.details
    }
}

fn fetch_current_state_version(workspace_name_suffix: &str) -> Result<String, Box<dyn Error>> {
    let terraform_organization = terraform_organization();
    let workspace_name = format!("{}_{}", bitte_cluster(), workspace_name_suffix);
    let workspace_id = workspace_id(terraform_organization.as_str(), workspace_name.as_str())?;
    current_state_version(&workspace_id)
}

fn current_state_version_output(state_id: &str) -> Result<HttpWorkspaceStateValue, Box<dyn Error>> {
    let mut client = terraform_client();
    let current_state_version_output: Result<HttpWorkspaceState, restson::Error> =
        client.get(state_id);
    match current_state_version_output {
        Ok(output) => Ok(output.data.attributes.value),
        Err(e) => Err(e.into()),
    }
}

fn workspace_id(organization: &str, workspace: &str) -> Result<String, Box<dyn Error>> {
    let mut client = terraform_client();
    let params = (organization, workspace);
    let workspace: Result<HttpWorkspace, restson::Error> = client.get(params);
    match workspace {
        Ok(workspace) => Ok(workspace.data.id),
        Err(e) => Err(e.into()),
    }
}

fn terraform_client() -> RestClient {
    let mut client =
        RestClient::new("https://app.terraform.io").expect("Couldn't create RestClient");
    let token =
        terraform_token().expect("Make sure you are logged into terraform: run `terraform login`");
    client
        .set_header("Authorization", format!("Bearer {}", token).as_str())
        .expect("Coudln't set Authorization header");
    client
        .set_header("Content-Type", "application/vnd.api+json")
        .expect("Couldn't set Content-Type header");
    client
}

fn terraform_token() -> Result<String, Box<dyn Error>> {
    let creds = parse_terraform_credentials();
    let c = &creds.credentials["app.terraform.io"];
    let token = &c.token;
    Ok(token.to_string())
}

fn parse_terraform_credentials() -> TerraformCredentialFile {
    let exp = &tilde("~/.terraform.d/credentials.tfrc.json").to_string();
    let path = Path::new(exp);
    let file = File::open(path).expect(format!("Couldn't read {}", exp).as_str());
    let reader = BufReader::new(file);
    let creds: TerraformCredentialFile =
        serde_json::from_reader(reader).expect(format!("Couldn't parse {}", exp).as_str());
    creds
}

fn terraform_organization() -> String {
    env::var("TERRAFORM_ORGANIZATION")
        .expect("TERRAFORM_ORGANIZATION environment variable must be set")
}

async fn wait_for_ssh(ip: String) {
    let addr = format!("{}:22", ip);

    for i in 0..120 {
        let stream = TcpStream::connect(addr.clone());
        let t = timeout(Duration::from_millis(10000), stream);
        match t.await {
            Ok(o) => match o {
                Ok(_) => {
                    return;
                }
                Err(ee) => {
                    if i >= 120 {
                        println!("error while connecting: {}", ee);
                    }
                }
            },
            Err(e) => println!("Waiting for {} to respond: {}", addr, e),
        }
    }
}

fn check_cmd(cmd: &mut Command) {
    println!("run: {:?}", cmd);
    cmd.status()
        .expect(format!("failed to run: {:?}", cmd).as_str());
}
