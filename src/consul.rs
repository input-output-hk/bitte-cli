use crate::types::ConsulAclTokenRead;
use crate::Result;

use super::sh;

// TODO: check that we have developer or admin policies
pub fn consul_token() -> Result<String> {
    match sh(execute::command_args!(
        "consul", "acl", "token", "read", "-self", "-format", "json"
    )) {
        Ok(output) => {
            let read: ConsulAclTokenRead = serde_json::from_str(output.as_str())?;
            Ok(read.secret_id)
        }
        Err(_err) => issue_consul_token(),
    }
}

fn issue_consul_token() -> Result<String> {
    sh(execute::command_args!(
        "vault",
        "read",
        "-field",
        "token",
        "consul/creds/developer"
    ))
}

/*
Example output of `consul acl token read -self -format json`:

{
    "CreateIndex": 10407178,
    "ModifyIndex": 10407178,
    "AccessorID": "bd49873d-055b-c879-d25a-ebcfe4a9088b",
    "SecretID": "ffffffff-aaaa-2222-bbbb-111111111111",
    "Description": "Vault developer aws-max.mustermann 1614860391931413618",
    "Policies": [
        {
            "ID": "8620f387-dd99-b28f-961b-7eb1603a0d76",
            "Name": "developer"
        }
    ],
    "Local": false,
    "CreateTime": "2021-03-04T12:19:51.932083652Z",
    "Hash": "6uzWRYtYfw4elm32i5K0NFrg353DeQiJ4jew4edkwQw="
}
*/
