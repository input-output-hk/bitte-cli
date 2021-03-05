use std::{
    env,
    io::Write,
    process::{exit, Command, ExitStatus},
};

use anyhow::{Context, Result};
use bitte_lib::{
    consul::consul_token,
    nomad::nomad_token,
    sh,
    types::{
        CueRender, NomadDeployment, NomadDeploymentStatus, NomadEvaluation, NomadJobPlan,
        NomadJobPlanDiff, NomadJobPlanField, NomadJobPlanObject, NomadJobPlanType, NomadJobRun,
        VaultTokenLookup,
    },
};
use clap::ArgMatches;
use colored::*;
use restson::RestClient;

pub(crate) async fn run(sub: &ArgMatches) -> Result<()> {
    let namespace: String = sub.value_of_t_or_exit("namespace");
    env::set_var("NOMAD_NAMESPACE", &namespace);
    Ok(())
}

pub(crate) async fn plan(sub: &ArgMatches) -> Result<()> {
    let namespace: String = sub.value_of_t_or_exit("namespace");
    let job: String = sub.value_of_t_or_exit("job");

    env::set_var("NOMAD_NAMESPACE", &namespace);

    let vault_token: String = vault_token()?;
    env::set_var("VAULT_TOKEN", &vault_token);

    let nomad_token = nomad_token()?;
    env::set_var("NOMAD_TOKEN", &nomad_token);

    let consul_token = consul_token()?;
    env::set_var("CONSUL_HTTP_TOKEN", &consul_token);

    let output = sh(execute::command_args!(
        "cue",
        "-t",
        format!("namespace={}", namespace),
        "-t",
        format!("job={}", job),
        "render"
    ))?;

    let mut render: CueRender =
        serde_json::from_str(output.as_str()).context("couldn't parse CUE export")?;
    // render.job.consul_token = Some(consul_token);
    render.diff = Some(true);

    let mut client = nomad_client(&nomad_token, &vault_token)?;
    let plan: NomadJobPlan = client.post_capture(render.job.id.as_str(), &render)?;

    println!("Running this job will make following changes:");

    diff(plan.diff);

    println!("The job modify index is: {}", plan.job_modify_index);

    ask_for_consent()?;

    render.diff = None;
    render.enforce_index = Some(true);
    render.job_modify_index = Some(plan.job_modify_index);
    let run: NomadJobRun = client.post_capture((), &render)?;
    println!("The EvalID is: {:?}", run.eval_id);

    loop {
        let evaluation: NomadEvaluation = client.get(run.eval_id.as_str())?;
        println!("evaluation: {:?}", &evaluation);

        match (evaluation.status.as_str(), &evaluation.deployment_id) {
            ("pending", _) => std::thread::sleep(std::time::Duration::from_secs(1)),
            ("complete", Some(deployment_id)) => loop {
                let deployment: NomadDeployment = client.get(deployment_id.as_str())?;

                match deployment.status {
                    NomadDeploymentStatus::Running => {
                        println!("{}", deployment.status_description.yellow());
                        std::thread::sleep(std::time::Duration::from_secs(1))
                    }
                    NomadDeploymentStatus::Complete => {
                        println!("{} {:?}", "Deployment complete!".green(), deployment);
                        return Ok(());
                    }
                    NomadDeploymentStatus::Successful => {
                        println!("{} {:?}", "Deployment successful!".green(), deployment);
                        return Ok(());
                    }
                    NomadDeploymentStatus::Failed => {
                        println!("{} {:?}", "Deployment failed!".red(), deployment);
                        std::thread::sleep(std::time::Duration::from_secs(10))
                    }
                }
            },
            (_, _) => {
                println!("evaluation: {:?}", evaluation);
                exit(1)
            }
        }
    }
}

fn ask_for_consent() -> Result<()> {
    print!("Do you want to apply these changes? (yes|no): ");
    std::io::stdout().flush()?;

    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;

    if line != "yes\n" {
        exit(0)
    }

    Ok(())
}

fn diff(diff: NomadJobPlanDiff) {
    println!("{}:", diff.id);
    diff_fields(2, &diff.fields);
    diff_objects(2, &diff.objects);

    for task_group in diff.task_groups {
        println!("{:>2} {}:", "", task_group.name);
        diff_objects(2, &task_group.objects);
        diff_fields(2, &task_group.fields);
        diff_objects(2, &task_group.tasks);
    }
}

fn diff_field(indent: usize, field: &NomadJobPlanField) {
    print_annotations(indent + 4, &field.annotations);

    let old = match redact(field.name.as_str(), field.old.as_str()) {
        "" => "null",
        o => o,
    };

    let new = match redact(field.name.as_str(), field.new.as_str()) {
        "" => "null",
        o => o,
    };

    match field.field_type {
        NomadJobPlanType::Added => println!(
            "{:>width$} {}: {}",
            "+".green(),
            field.name,
            new.green(),
            width = indent
        ),
        NomadJobPlanType::Deleted => println!(
            "{:>width$} {}: {}",
            "-".red(),
            field.name,
            old.red(),
            width = indent
        ),
        NomadJobPlanType::Edited => println!(
            "{:>width$} {}: {} -> {}",
            "~".yellow(),
            field.name,
            old.red(),
            new.green(),
            width = indent
        ),
        NomadJobPlanType::None => (),
    }
}

fn redact<'a>(name: &str, value: &'a str) -> &'a str {
    match name {
        "ConsulToken" => "<redacted>",
        _ => value,
    }
}

fn diff_fields(indent: usize, fields: &Option<Vec<NomadJobPlanField>>) {
    if let Some(fields) = fields {
        for field in fields {
            diff_field(indent + 2, field)
        }
    }
}

fn diff_object(indent: usize, obj: &NomadJobPlanObject) {
    diff_fields(indent + 2, &obj.fields);
    diff_objects(indent + 2, &obj.objects);
}

fn diff_objects(indent: usize, objects: &Option<Vec<NomadJobPlanObject>>) {
    if let Some(objects) = objects {
        for obj in objects {
            println!("{:>width$} {}:", "", obj.name, width = indent + 2);
            print_annotations(indent + 6, &obj.annotations);
            diff_object(indent + 2, &obj);
        }
    }
}

fn print_annotations(indent: usize, annotations: &Option<Vec<String>>) {
    if let Some(annotations) = annotations {
        for annotation in annotations {
            println!(
                "{:>width$} {}",
                "!".purple(),
                annotation.purple(),
                width = indent
            )
        }
    }
}

fn nomad_client(nomad_token: &String, vault_token: &String) -> Result<RestClient> {
    let nomad_addr = env::var("NOMAD_ADDR")?;
    let mut client = RestClient::new(&nomad_addr)?;
    client.set_header("X-Nomad-Token", nomad_token)?;
    client.set_header("X-Vault-Token", vault_token)?;
    Ok(client)
}

fn vault_token() -> Result<String> {
    if let Ok(token) = lookup_current_vault_token() {
        return Ok(token);
    }
    vault_login()?;
    lookup_current_vault_token()
}

fn lookup_current_vault_token() -> Result<String> {
    let mut cmd = Command::new("vault");
    let full = cmd.args(vec!["token", "lookup", "-format=json"]);
    let output = full.output().context("vault token lookup failed")?;
    let lookup: VaultTokenLookup = serde_json::from_slice(output.stdout.as_slice())?;
    Ok(lookup.data.id)
}

// TODO: give option to login using aws?
fn vault_login() -> Result<ExitStatus> {
    Command::new("vault")
        .args(vec![
            "login",
            "-method=github",
            "-path=github-employees",
            "-no-print",
        ])
        .status()
        .context("vault login failed")
}
