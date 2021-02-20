use rusoto_autoscaling::{AutoScalingGroupNamesType, Autoscaling, AutoscalingClient};
use rusoto_ec2::{DescribeInstancesRequest, Ec2, Ec2Client};
use std::str::FromStr;

pub async fn asg_info(tf_arn: &str, region_name: &str) -> Vec<rusoto_autoscaling::Instance> {
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

pub async fn instance_info(instance_id: &str, region_name: &str) -> Vec<rusoto_ec2::Instance> {
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
