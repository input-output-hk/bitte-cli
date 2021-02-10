use prettytable::{cell, row, Table};
use rusoto_autoscaling::{AutoScalingGroupNamesType, Autoscaling, AutoscalingClient};
use rusoto_ec2::{DescribeInstancesRequest, Ec2, Ec2Client};
use std::str::FromStr;

use super::current_state_version_output;

pub(crate) async fn cli_info_print(current_state_version: String) {
    let output = current_state_version_output(&current_state_version).unwrap();

    let mut instance_table = Table::new();
    instance_table.add_row(row!["Name", "Type", "FlakeAttr", "Private IP", "Public IP"]);

    for (key, val) in output.instances.iter() {
        instance_table.add_row(row![
            key,
            val.instance_type,
            val.flake_attr,
            val.private_ip,
            val.public_ip,
        ]);
    }

    instance_table.printstd();

    if let Some(asgs) = output.asgs {
        let mut asg_table = Table::new();

        asg_table.add_row(row![
            "Id",
            "Type",
            "AZ",
            "State",
            "Status",
            "Protected",
            "PrivateIp",
            "PublicIp"
        ]);

        for (_key, val) in asgs.iter() {
            let info = asg_info(val.arn.as_str(), val.region.as_str()).await;
            for asgi in info {
                // TODO: rewrite to take all required instance ids as argument to save time
                let ii = instance_info(asgi.instance_id.as_str(), val.region.as_str()).await;
                let iii = ii[0].clone();

                asg_table.add_row(row![
                    asgi.instance_id,
                    asgi.instance_type.unwrap_or("".to_string()),
                    asgi.availability_zone,
                    asgi.lifecycle_state,
                    asgi.health_status,
                    asgi.protected_from_scale_in,
                    iii.public_ip_address.unwrap_or("".to_string()),
                    iii.private_ip_address.unwrap_or("".to_string()),
                ]);
                // asg_table.add_row(row![key, val.instance_type, val.flake_attr, val.count,]);
            }
        }

        asg_table.printstd();
    };
}

pub(crate) async fn asg_info(tf_arn: &str, region_name: &str) -> Vec<rusoto_autoscaling::Instance> {
    let region = rusoto_core::Region::from_str(region_name).expect("Region not found");
    let client = AutoscalingClient::new(region);
    let request = AutoScalingGroupNamesType::default();
    let response = client
        .describe_auto_scaling_groups(request)
        .await
        .expect("Unable to fetch autoscaling groups info");
    let iter = response.auto_scaling_groups.into_iter();
    let matching = iter.filter(|asg| Some(tf_arn.to_string()) == asg.auto_scaling_group_arn);
    matching
        .flat_map(|asg| asg.instances.unwrap_or_else(|| vec![]))
        .collect()
}

pub(crate) async fn instance_info(instance_id: &str, region_name: &str) -> Vec<rusoto_ec2::Instance> {
    let region = rusoto_core::Region::from_str(region_name).expect("Region not found");
    let client = Ec2Client::new(region);
    let request = DescribeInstancesRequest {
        instance_ids: Some(vec![instance_id.to_string()]),
        dry_run: None,
        filters: None,
        max_results: None,
        next_token: None,
    };

    let response = client
        .describe_instances(request)
        .await
        .expect("Unable to fetch EC2 Instance info");

    let iter = response.reservations.into_iter();
    iter.flat_map(|reservations| {
        reservations
            .into_iter()
            .flat_map(|reservation| reservation.instances.unwrap_or_else(|| vec![]))
    })
    .collect()
}
