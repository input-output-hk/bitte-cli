use std::{
    error::Error,
    fs,
    process::Command,
};

use log::{debug, info};

use super::{
    bitte_cluster, terraform_client, terraform_organization,
    types::{
        HttpPostWorkspaceData, HttpPostWorkspaces, HttpWorkspaceCurrentStateVersion,
        HttpWorkspaceData, HttpWorkspaceDataAttributes, HttpWorkspaces,
    },
};

pub fn workspace_list() -> Result<Vec<HttpWorkspaceData>, Box<dyn Error>> {
    let mut client = terraform_client();
    let workspaces: Result<HttpWorkspaces, restson::Error> =
        client.get(terraform_organization().as_str());
    match workspaces {
        Ok(list) => Ok(list.data),
        Err(e) => Err(e.into()),
    }
}

pub fn prepare(workspace: String) {
    info!("prepare terraform");
    // To work on Darwin, we need to pass the current system

    generate_terraform_config(&workspace);

    let original = workspace_show();
    info!("original: {}, workspace: {}", original, workspace);
    if original != workspace {
        let list: Vec<String> = workspace_list()
            .unwrap_or_else(|_| vec![])
            .iter()
            .map(|w| w.attributes.name.clone())
            .collect();
        debug!("{:?}", list);
        let workspace_fullname = format!("{}_{}", bitte_cluster(), workspace);
        if !list.contains(&workspace_fullname) {
            workspace_new(&workspace_fullname)
                .expect(format!("Failed to create workspace {}", workspace).as_str());
        }
        workspace_select(workspace);
    }
    init()
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

fn workspace_new(workspace: &str) -> Result<(), Box<dyn Error>> {
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

fn init() {
    println!("run: terraform init");
    Command::new("terraform")
        .arg("init")
        .status()
        .expect("terraform init failed");
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
