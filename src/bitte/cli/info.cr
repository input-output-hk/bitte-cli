require "tablo"

module Bitte
  class CLI
    register_sub_command info : Info, description: "Show information about clusters and instances"

    class Info < Admiral::Command
      include Helpers

      define_help description: "Show information about clusters and instances"

      def run
        client = AWS::Client.new

        data = client.auto_scaling_groups.auto_scaling_groups.flat_map { |asg|
          instances = client.describe_instances(
            asg.instances.map(&.instance_id)
          ).reservations.map(&.instances).flatten

          asg.instances.map do |asgi|
            i = instances.find{|instance| instance.instance_id == asgi.instance_id }
            if i
              [
                asgi.instance_id,
                asgi.instance_type,
                asgi.availability_zone,
                asgi.lifecycle_state,
                asgi.health_status,
                asgi.launch_configuration_name || "",
                i.private_ip_address || "",
                i.public_ip_address || "",
              ]
            else
              raise "Can't find #{asgi.instance_id}"
            end
          end
        }

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

        puts "Auto Scaling Groups"
        puts table
      end

      def profile
        parent.flags.as(CLI::Flags).profile
      end

      def region
        parent.flags.as(CLI::Flags).region
      end

      def cluster_name
        parent.flags.as(CLI::Flags).cluster
      end
    end
  end
end
