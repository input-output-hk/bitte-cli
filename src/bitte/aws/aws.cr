require "./aws_types.cr"

module AWS
  class Client
    property region : String

    def initialize(@region)
    end

    def aws(cmd, subcmd, args = [] of String)
      aws_args = [
        "--region", region,
        "--output", "json",
        cmd, subcmd,
      ] + args

      output = IO::Memory.new
      Process.run("aws", args: aws_args, output: output, error: STDERR)
      output.to_s
    end

    def auto_scaling_groups
      AWS::Types::Autoscaling::DescribeAutoScalingGroups.from_json(
        aws("autoscaling", "describe-auto-scaling-groups"))
    end

    def describe_instances
      AWS::Types::EC2::DescribeInstances.from_json(
        aws("ec2", "describe-instances"))
    end

    def describe_instances(instance_ids)
      AWS::Types::EC2::DescribeInstances.from_json(
        aws("ec2", "describe-instances", ["--instance-ids"] + instance_ids))
    end

    def list_keys
      AWS::Types::KMS::ListKeys.from_json(
        aws("kms", "list-keys"))
    end

    def reap(instance_id)
      aws("autoscaling", "set-instance-health", [
        "--instance-id", instance_id, "--health-status", "Unhealthy",
      ])
    end

    def describe_images(**filters)
      kvs = filters.map { |k, v| %(Name=#{k},Values="#{v}") }
      AWS::Types::DescribeImages::Images.from_json(
        aws("ec2", "describe-images", ["--filters"] + kvs))
    end

    def deregister_image(image_id : String)
      aws("ec2", "deregister-image", ["--image-id", image_id])
    end

    def delete_snapshot(snapshot_id : String)
      aws("ec2", "deregister-image", ["--snapshot-id", snapshot_id])
    end

    def s3_rm(url, recursive = false)
      if recursive
        aws("s3", "rm", ["--recursive", url])
      else
        aws("s3", "rm", [url])
      end
    end
  end
end
