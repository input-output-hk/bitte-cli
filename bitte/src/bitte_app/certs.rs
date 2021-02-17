use std::{env, fs, process::Command};

use clap::ArgMatches;
use serde::Deserialize;

use super::check_cmd;

pub async fn cli_certs(sub: &ArgMatches) {
    let domain: String = sub.value_of_t_or_exit("domain");
    env::set_var("VAULT_ADDR", format!("https://vault.{}", domain));
    env::set_var("VAULT_CACERT", "secrets/ca.pem");
    env::set_var("VAULT_FORMAT", "json");
    env::set_var("VAULT_SKIP_VERIFY", "true");

    vault_login();
    write_issuing_ca(&domain);
    sign_intermediate();
}

fn sign_intermediate() {
    let ca_pem_orig = fs::read_to_string("secrets/ca.pem").expect("Couldn't read ca.pem");
    let ca_pem = ca_pem_orig.trim();
    let cert_pem_orig = fs::read_to_string("secrets/cert.pem").expect("Couldn't read cert.pem");
    let cert_pem = cert_pem_orig.trim();

    let output = cmd_output(Command::new("cfssl").args(vec![
        "sign",
        "-ca",
        "secrets/ca.pem",
        "-ca-key",
        "secrets/ca-key.pem",
        "-hostname",
        "vault.service.consul",
        "-config",
        ca_config_file().as_str(),
        "-profile",
        "intermediate",
        "secrets/issuing-ca.csr",
    ]));

    let issuing_csr_container: Cert =
        serde_json::from_str(output.as_str()).expect("couldn't parse output from cfssl");
    let issuing_pem = issuing_csr_container.cert.trim();

    fs::write("secrets/issuing.pem", issuing_pem).expect("Couldn't write issuing.pem");
    fs::write(
        "secrets/issuing_full.pem",
        vec![issuing_pem, ca_pem].join("\n"),
    )
    .expect("Couldn't write issuing_full.pem");

    check_cmd(Command::new("vault").args(&[
        "write",
        "pki/intermediate/set-signed",
        "certificate=@secrets/issuing_full.pem",
    ]));

    let full = vec![cert_pem, issuing_pem, ca_pem].join("\n");
    fs::write("secrets/full.pem", &full).expect("Couldn't write full.pem");

    check_cmd(
        Command::new("sops")
            .arg("--set")
            .arg(format!(
                r#"["full"] {}"#,
                serde_json::to_string(&full).expect("Couldn't generate JSON for full.pem")
            ))
            .arg("encrypted/cert.json"),
    )
}

fn ca_config_file() -> String {
    let json = serde_json::json!({
        "signing": {
            "default": {
                "expiry": "87600h",
            },
            "profiles": {
                "default": {
                    "usages": ["signing", "key encipherment", "server auth", "client auth"],
                    "expiry": "8760h",
                },

                "intermediate": {
                    "usages":        ["signing", "key encipherment", "cert sign", "crl sign"],
                    "expiry":        "43800h",
                    "ca_constraint": {"is_ca": true},
                },
            },
        },
    });
    let location = "secrets/ca-config.json";
    fs::write(location, json.to_string()).expect("couldn't write ca-config.json");
    location.to_string()
}

fn write_issuing_ca(domain: &String) {
    let issuing_ca = vault_issuing_ca(&domain);
    let csr_container: CSR = serde_json::from_str(&issuing_ca).expect("Couldn't parse issuing CA");
    fs::write("secrets/issuing-ca.csr", csr_container.data.csr)
        .expect("Couldn't write issuing-ca.csr");
}

fn vault_issuing_ca(domain: &String) -> String {
    cmd_output(Command::new("vault").args(&[
        "write",
        "pki/intermediate/generate/internal",
        format!(r#"common_name="vault.{}""#, domain).as_str(),
    ]))
}

fn vault_login() {
    check_cmd(Command::new("vault").args(&["login", "-method", "aws", "-no-print"]));
}

fn cmd_output(cmd: &mut Command) -> String {
    let output = cmd
        .output()
        .expect("unable to fetch intermediate signing request");
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[derive(Deserialize)]
struct CSR {
    data: CSRValue,
}

#[derive(Deserialize)]
struct CSRValue {
    csr: String,
}

#[derive(Deserialize)]
struct Cert {
    cert: String,
}
