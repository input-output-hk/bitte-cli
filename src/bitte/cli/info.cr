require "tablo"

module Bitte
  class CLI
    register_sub_command info : Info, description: "Show information about clusters and instances"

    class Info < Admiral::Command
      include Helpers

      define_help short: "h", description: "Show information about clusters and instances"

      property cluster : TerraformCluster?

      def run
        puts_core
        puts_asgs
      end

      def puts_core
        puts "Core"

        data = core_cluster.instances.map do |_, instance|
          [
            instance.name,
            instance.instance_type,
            instance.flake_attr,
            instance.private_ip,
            instance.public_ip,
          ]
        end

        table = Tablo::Table.new(data) do |t|
          t.add_column("Name") { |n| n[0] }
          t.add_column("Type") { |n| n[1] }
          t.add_column("FlakeAttr") { |n| n[2] }
          t.add_column("Private IP") { |n| n[3] }
          t.add_column("Public IP") { |n| n[4] }
        end

        table.shrinkwrap!
        puts table
      end

      def puts_asgs
        puts "Auto Scaling Groups"

        asgs = cluster.asgs || Hash(String, TerraformCluster::ASG).new

        data = asgs.flat_map do |_, asg|
          asg.instances.map do |instance|
            [
              instance.name,
              instance.type,
              instance.availability_zone,
              instance.lifecycle_state,
              instance.health_status,
              instance.launch_configuration_name || "",
              instance.private_ip || "",
              instance.public_ip || "",
            ]
          end
        end

        table = Tablo::Table.new(data) do |t|
          t.add_column("InstanceId") { |n| n[0] }
          t.add_column("InstanceType") { |n| n[1] }
          t.add_column("AvailabilityZone") { |n| n[2] }
          t.add_column("LifecycleState") { |n| n[3] }
          t.add_column("HealthStatus") { |n| n[4] }
          t.add_column("LaunchConfigurationName") { |n| n[5] }
          t.add_column("PrivateIp") { |n| n[6] }
          t.add_column("PublicIp") { |n| n[7] }
        end

        table.shrinkwrap!
        puts table
      end

      def cluster
        @cluster ||= TerraformCluster.load("clients")
      end

      def core_cluster
        @cluster ||= TerraformCluster.load("core")
      end

      def cluster_name
        parent.flags.as(CLI::Flags).cluster
      end
    end
  end
end
