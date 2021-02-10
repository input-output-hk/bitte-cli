use std::process::Command;

use clap::ArgMatches;

use super::{
    current_state_version_output, fetch_current_state_version,
    info::{asg_info, instance_info},
};

pub(crate) async fn cli_ssh(sub: &ArgMatches) {
    let needle: String = sub.value_of_t("host").expect("host argument must be given");
    let mut args = sub.values_of_lossy("args").unwrap_or(vec![]);

    let user_host = format!("root@{}", find_host(needle).await);
    let mut flags = vec!["-x".to_string(), "-p".into(), "22".into(), user_host.into()];
    flags.append(args.as_mut());
    if args.len() > 0 {
        flags.append(&mut vec!["-t".to_string()]);
    }
    let ssh_args = flags.into_iter();

    let mut cmd = Command::new("ssh");
    let cmd_with_args = cmd.args(ssh_args);
    println!("cmd: {:?}", cmd_with_args);

    cmd.spawn()
        .expect("ssh command failed")
        .wait()
        .expect("ssh command didn't finish?");
}

async fn find_host(needle: String) -> String {
    let current_state_version = fetch_current_state_version("clients")
        .or_else(|_| fetch_current_state_version("core"))
        .expect("Coudln't fetch clients or core workspaces");

    let output = current_state_version_output(&current_state_version)
        .expect("Problem loading state version from terraform");

    for instance in output.instances.values().into_iter() {
        if instance.private_ip == needle || instance.public_ip == needle || instance.name == needle
        {
            return instance.public_ip.clone();
        }
    }

    if let Some(asgs) = output.asgs {
        for (_, asg) in asgs {
            let asg_infos = asg_info(asg.arn.as_str(), asg.region.as_str()).await;
            for asg_info in asg_infos {
                let instance_infos =
                    instance_info(asg_info.instance_id.as_str(), asg.region.as_str()).await;
                for instance_info in instance_infos {
                    let public_ip = instance_info.public_ip_address;
                    if vec![
                        instance_info.instance_id.unwrap_or("".into()),
                        instance_info.public_dns_name.unwrap_or("".into()),
                        public_ip.clone().unwrap_or("".into()),
                        instance_info.private_dns_name.unwrap_or("".into()),
                        instance_info.private_ip_address.unwrap_or("".into()),
                    ]
                    .contains(&needle)
                    {
                        if let Some(ip) = public_ip {
                            return ip;
                        }
                    }
                }
            }
        }
    }

    needle
}
