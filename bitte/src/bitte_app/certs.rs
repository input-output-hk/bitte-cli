use std::{env, process::Command};

use clap::ArgMatches;

use super::check_cmd;

pub(crate) async fn cli_certs(sub: &ArgMatches) {
    let domain: String = sub.value_of_t("domain").expect("domain flag not set");
    env::set_var("VAULT_ADDR", format!("https://vault.{}", domain));
    env::set_var("VAULT_CACERT", "secrets/ca.pem");
    env::set_var("VAULT_FORMAT", "json");
    env::set_var("VAULT_SKIP_VERIFY", "true");

    vault_login();
    let issuing_ca = vault_issuing_ca(&domain);
}

fn vault_issuing_ca(domain: &String) -> String {
    cmd_output(Command::new("vault").args(&[
        "write",
        "pki/intermediate/generate/internal",
        format!("common_name=\"vault.{}\"", domain).as_str(),
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
