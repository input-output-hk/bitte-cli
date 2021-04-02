use std::{collections::HashMap, env, io::Write, process::{exit, Command, ExitStatus}};

use anyhow::{anyhow, Context, Result};
use bitte_lib::{
    consul::consul_token,
    nomad::{nomad_token, NomadEvent},
    sh,
    types::{
        CueRender, NomadDeployment, NomadEvaluation, NomadJobPlan, NomadJobPlanDiff,
        NomadJobPlanField, NomadJobPlanObject, NomadJobPlanType, NomadJobRun, VaultTokenLookup,
    },
};
use clap::ArgMatches;
use colored::*;
use hyper::{body::HttpBody, Client};
use hyper_tls::HttpsConnector;
use restson::RestClient;

pub async fn events(_sub: &ArgMatches) -> Result<()> {
    let nomad_addr = env::var("NOMAD_ADDR")?;
    let url: hyper::Uri = format!(
        "{}/v1/event/stream?topic=Evaluation&topic=Job&topic=Deployment&topic=Allocation&namespace=mantis-testnet",
        nomad_addr
    )
    .parse()?;
    println!("GET {}", url);

    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);
    let request = hyper::Request::builder()
        .method("GET")
        .header("X-Nomad-Token", nomad_token()?)
        .uri(url)
        .body(hyper::Body::empty())?;

    let mut response = client.request(request).await.unwrap();

    let mut buf = Vec::<u8>::new();

    while let Some(chunk) = response.body_mut().data().await {
        for byte in chunk? {
            if byte == 10 {
                let c = String::from_utf8_lossy(buf.as_slice()).to_string();
                let stream =
                    serde_json::Deserializer::from_str(&c.as_str()).into_iter::<NomadEvent>();

                for json in stream {
                    match json {
                        Ok(value) => {
                            println!("{}", value)
                        }
                        Err(err) => {
                            println!("error: {:?}", err)
                        }
                    }
                }
                buf.truncate(0);
            } else {
                buf.push(byte);
            }
        }
    }

    Ok(())
}

/*
mostly used for debugging:

pub(crate) async fn events(_sub: &ArgMatches) -> Result<()> {
    let file = std::fs::read("events_sorted.json")?;
    let mut buf = Vec::<u8>::new();
    let mut line = 0;
    for byte in file.iter() {
        if *byte == 10 {
            line = line + 1;
            println!("{}", line);
            let c = String::from_utf8_lossy(buf.as_slice()).to_string();
            let stream = serde_json::Deserializer::from_str(&c.as_str()).into_iter::<NomadEvent>();

            for json in stream {
                match json {
                    Ok(value) => {
                        println!("{}", value)
                    }
                    Err(err) => {
                        println!("error: {:?}", err)
                    }
                }
            }
            buf.truncate(0);
        } else {
            buf.push(*byte);
        }
    }

    Ok(())
}
*/

pub(crate) async fn plan(sub: &ArgMatches) -> Result<()> {
    let namespace: String = sub.value_of_t_or_exit("namespace");
    let job_arg: Result<String, clap::Error> = sub.value_of_t("job");

    sh(execute::command_args!("cue", "vet", "-c", "./..."))
        .context("failure during: `cue vet -c ./...`")?;

    env::set_var("NOMAD_NAMESPACE", &namespace);

    let vault_token: String = vault_token()?;
    env::set_var("VAULT_TOKEN", &vault_token);

    let nomad_token = nomad_token()?;
    env::set_var("NOMAD_TOKEN", &nomad_token);

    let consul_token = consul_token()?;
    env::set_var("CONSUL_HTTP_TOKEN", &consul_token);

    match job_arg {
        Ok(job) => plan_job(namespace, job).await,
        Err(_) => plan_jobs(&namespace).await,
    }
}

#[derive(Debug, serde::Deserialize)]
struct CueExport {
    rendered: HashMap<String, HashMap<String, serde_json::Value>>
}

async fn plan_jobs(namespace: &String) -> Result<()> {
    let output = sh(execute::command_args!("cue", "export"))?;
    let export : CueExport = serde_json::from_str(output.as_str()).context("Couldn't parse CUE export")?;

    if let Some(n) = export.rendered.get(namespace) {
        for job in n.keys() {
            plan_job(namespace.to_string(), job.to_string()).await?;
        }
    }

    Ok(())
}

async fn plan_job(namespace: String, job: String) -> Result<()> {
    let output = sh(execute::command_args!(
        "cue",
        "-t",
        format!("namespace={}", namespace),
        "-t",
        format!("job={}", job),
        "render"
    ))?;

    let mut render: CueRender =
        serde_json::from_str(output.as_str()).context("couldn't parse CUE render")?;
    // render.job.consul_token = Some(consul_token);
    render.diff = Some(true);

    let mut client = nomad_client()?;
    let plan: Result<NomadJobPlan, restson::Error> =
        client.post_capture(render.job.id.as_str(), &render);

    match plan {
        Ok(p) => execute_plan(&mut client, &mut render, p),
        Err(e) => match e {
            restson::Error::SerializeParseError(error) => Err(anyhow!(error)),
            restson::Error::DeserializeParseError(error, string) => {
                println!("{}", string);
                Err(anyhow!(error))
            }
            restson::Error::HyperError(error) => Err(anyhow!(error)),
            restson::Error::IoError(error) => Err(anyhow!(error)),
            restson::Error::HttpError(_, error) => Err(anyhow!(error)),
            other => Err(anyhow!(other)),
        },
    }
}

fn execute_plan(client: &mut RestClient, render: &mut CueRender, plan: NomadJobPlan) -> Result<()> {
    println!("Running this job will make following changes:");

    diff(plan.diff);

    println!("The job modify index is: {}", plan.job_modify_index);

    ask_for_consent()?;

    render.diff = None;
    render.enforce_index = Some(true);
    render.job_modify_index = Some(plan.job_modify_index);
    let run: NomadJobRun = client.post_capture((), render)?;
    println!("The EvalID is: {:?}", run.eval_id);

    loop {
        let evaluation: NomadEvaluation = client.get(run.eval_id.as_str())?;
        println!("evaluation: {:?}", &evaluation);

        match (evaluation.status.as_str(), &evaluation.deployment_id) {
            ("pending", _) => std::thread::sleep(std::time::Duration::from_secs(1)),
            ("complete", Some(deployment_id)) => {
                let mut deployment: NomadDeployment = client.get(deployment_id.as_str())?;
                deployment.display();

                loop {
                    let new_deployment = client.get(deployment_id.as_str())?;
                    if deployment == new_deployment {
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        continue;
                    } else {
                        deployment = new_deployment;
                    }

                    deployment.display();

                    if deployment.is_done() {
                        return Ok(());
                    }
                }
            }
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

fn nomad_client() -> Result<RestClient> {
    let nomad_addr = env::var("NOMAD_ADDR")?;
    let mut client = RestClient::new(&nomad_addr)?;
    let nomad_token = env::var("NOMAD_TOKEN")?;
    let vault_token = env::var("VAULT_TOKEN")?;
    client.set_header("X-Nomad-Token", &nomad_token)?;
    client.set_header("X-Vault-Token", &vault_token)?;
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
