use std::{
    error::Error,
    fs,
    process::Command,
};

use clap::ArgMatches;

use super::{
    bitte_cluster, terraform_client, terraform_organization,
    types::{
        HttpPostWorkspaceData, HttpPostWorkspaces, HttpWorkspaceCurrentStateVersion,
        HttpWorkspaceData, HttpWorkspaceDataAttributes, HttpWorkspaces,
    },
};

pub async fn cli_tf_plan(workspace: String, sub: &ArgMatches) {
    let destroy: bool = sub.value_of_t("destroy").unwrap_or(false);
    let plan_file = format!("{}.plan", workspace);

    prepare_terraform(workspace);

    let mut cmd = Command::new("terraform");
    let mut full = cmd.arg("plan").arg("-out").arg(plan_file);
    if destroy {
        full = full.arg("-destroy");
    }

    println!("run: {:?}", full);
    full.status()
        .expect(format!("failed to run: {:?}", full).as_str());
}

pub async fn cli_tf_apply(workspace: String, _sub: &ArgMatches) {
    let plan_file = format!("{}.plan", workspace);

    prepare_terraform(workspace);

    let mut cmd = Command::new("terraform");
    let full = cmd.arg("apply").arg(plan_file);

    println!("run: {:?}", full);
    full.status()
        .expect(format!("failed to run: {:?}", full).as_str());
}

pub async fn cli_tf_workspaces(_workspace: String, _sub: &ArgMatches) {
    let list = tf_workspace_list();
    println!("{:?}", list)
}

fn tf_workspace_list() -> Result<Vec<HttpWorkspaceData>, Box<dyn Error>> {
    let mut client = terraform_client();
    let workspaces: Result<HttpWorkspaces, restson::Error> =
        client.get(terraform_organization().as_str());
    match workspaces {
        Ok(list) => Ok(list.data),
        Err(e) => Err(e.into()),
    }
}

fn prepare_terraform(workspace: String) {
    println!("prepare terraform");
    // To work on Darwin, we need to pass the current system

    generate_terraform_config(&workspace);

    let original = tf_workspace_show();
    println!("original: {}, workspace: {}", original, workspace);
    if original != workspace {
        let list: Vec<String> = tf_workspace_list()
            .unwrap_or_else(|_| vec![])
            .iter()
            .map(|w| w.attributes.name.clone())
            .collect();
        if !list.contains(&format!("{}_{}", bitte_cluster(), workspace)) {
            tf_workspace_new(&workspace)
                .expect(format!("Failed to create workspace {}", workspace).as_str());
        }
        tf_workspace_select(workspace);
    }
    tf_init()
}

fn generate_terraform_config(workspace: &String) {
    match Command::new("nix")
        .arg("-L")
        .arg("run")
        .arg(format!(
            ".#clusters.{}.{}.tf.{}.config",
            nix_current_system(),
            bitte_cluster(),
            workspace
        ))
        .status()
    {
        Ok(o) => {
            if !o.success() {
                println!("or_else");
                // For backwards compatibility
                Command::new("nix")
                    .arg("-L")
                    .arg("run")
                    .arg(format!(
                        ".#clusters.{}.tf.{}.config",
                        bitte_cluster(),
                        workspace
                    ))
                    .status()
                    .expect("failed to generate config");
            }
        }
        Err(e) => println!("error while generating config: {}", e),
    }
}

fn tf_workspace_new(workspace: &str) -> Result<(), Box<dyn Error>> {
    let mut client = terraform_client();
    let body = HttpPostWorkspaces {
        workspace_type: "workspace".to_string(),
        data: HttpPostWorkspaceData {
            attributes: HttpWorkspaceDataAttributes {
                name: workspace.to_string(),
                operations: false,
            },
        },
    };
    let result: Result<(), restson::Error> =
        client.post_with(terraform_organization().as_str(), &body, &[]);
    match result {
        Ok(r) => Ok(r),
        Err(e) => Err(e.into()),
    }
}

fn tf_workspace_show() -> String {
    match fs::read(".terraform/environment") {
        Ok(content) => String::from_utf8_lossy(&content).to_string(),
        Err(e) => {
            println!("error while reading workspace: {}", e);
            "default".to_string()
        }
    }
}

fn tf_workspace_select(workspace: String) {
    println!("run: terraform workspace select {}", workspace);
    Command::new("terraform")
        .arg("workspace")
        .arg("select")
        .arg(workspace)
        .status()
        .expect("terraform workspace select failed");
}

fn tf_init() {
    println!("run: terraform init");
    Command::new("terraform")
        .arg("init")
        .status()
        .expect("terraform init failed");
}

pub fn current_state_version(workspace_id: &str) -> Result<String, Box<dyn Error>> {
    let mut client = terraform_client();
    let current_state_version: Result<HttpWorkspaceCurrentStateVersion, restson::Error> =
        client.get(workspace_id);
    match current_state_version {
        Ok(version) => Ok(version.data.relationships.outputs.data[0].id.to_string()),
        Err(e) => Err(e.into()),
    }
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
