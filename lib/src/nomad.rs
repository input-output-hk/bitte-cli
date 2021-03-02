use super::handle_command_error;
use anyhow::{Context, Result};

// TODO: check that we have developer or admin policies
pub fn nomad_token() -> Result<String, anyhow::Error> {
    match handle_command_error(execute::command_args!("nomad", "acl", "token", "self")) {
        Ok(output) => {
            for line in output.lines() {
                let parts: Vec<&str> = line.splitn(2, '=').collect();
                let key = parts[0].trim();
                let value = parts[1].trim();
                if key == "Secret ID" {
                    return Ok(value.to_string());
                }
            }
            issue_nomad_token()
        }
        Err(_err) => issue_nomad_token(),
    }
}

fn issue_nomad_token() -> Result<String, anyhow::Error> {
    handle_command_error(execute::command_args!(
        "vault",
        "read",
        "-field",
        "secret_id",
        "nomad/creds/developer"
    ))
    .context("unable to fetch nomad token from vault")
}

/*
Example output from `nomad acl token self`:

Accessor ID  = 77777777-8888-9999-aaaa-bbbbbbbbbbbb
Secret ID    = 88888888-9999-aaaa-bbbb-cccccccccccc
Name         = vault-admin-aws-max.mustermann-0000000000000000000
Type         = client
Global       = false
Policies     = [admin]
Create Time  = 2021-02-24 15:25:25.0645971 +0000 UTC
Create Index = 2492001
Modify Index = 2492001
*/
