module Bitte
  class TerraformCluster
    include JSON::Serializable

    extend CLI::Helpers

    def self.load(workspace)
      cluster_name = ENV["BITTE_CLUSTER"]
      state = Terraform.current_state_version_output(tf_organization, cluster_name, workspace)

      outer = NamedTuple(data: NamedTuple(attributes: NamedTuple(value: TerraformCluster)))
        .from_json(state)

      outer[:data][:attributes][:value].tap do |cluster|
        next unless asgs = cluster.asgs
        asgs.each do |name, asg|
          asg.cluster = cluster
          asg.name = name
        end
      end
    end

    property asgs : Hash(String, ASG)?
    property flake : String
    property instances : Hash(String, Instance)?
    property kms : String
    property name : String
    property nix : String
    property region : String
    property roles : Roles
    property s3_cache : String?
    property s3_bucket : String?

    def instances
      @instances || Hash(String, Instance).new
    end

    def asgs
      @asgs || Hash(String, ASG).new
    end

    class ASG
      include JSON::Serializable

      property name : String?
      property cluster : TerraformCluster?
      property region : String
      property uid : String
      property arn : String

      @[JSON::Field(key: "flake-attr")]
      property flake_attr : String

      @[JSON::Field(key: "instance-type")]
      property instance_type : String

      record Instance,
         asg : ASG,
         name : String,
         type : String,
         availability_zone : String,
         lifecycle_state : String,
         health_status : String,
         launch_configuration_name : String?,
         private_ip : String,
         public_ip : String?,
         tags = Hash(String, String).new

      def instances
        asgs.flat_map { |asg|
          next unless asg.arn == arn

          instances = aws_client.describe_instances(
            asg.instances.map(&.instance_id)
          ).reservations.map(&.instances).flatten

          asg.instances.map do |asgi|
            i = instances.find { |instance| instance.instance_id == asgi.instance_id }
            next unless i

            tags = i.tags_hash

            if i && tags["Name"]? == self.name
              ASG::Instance.new(
                asg: self,
                name: asgi.instance_id,
                type: asgi.instance_type,
                availability_zone: asgi.availability_zone,
                health_status: asgi.health_status,
                lifecycle_state: asgi.lifecycle_state,
                launch_configuration_name: asgi.launch_configuration_name,
                private_ip: i.private_ip_address.not_nil!,
                public_ip: i.public_ip_address,
                tags: tags,
              )
            else
              raise "Can't find #{asgi.instance_id}"
            end
          end
        }.compact
      end

      def asgs
        aws_client.auto_scaling_groups.auto_scaling_groups
      end

      def aws_client
        AWS::Client.new(region)
      end

      def cluster
        @cluster.not_nil!
      end

      def name
        @name.not_nil!
      end
    end

    class Instance
      include JSON::Serializable

      property tags : Hash(String, String)
      property uid : String
      property name : String

      @[JSON::Field(key: "flake-attr")]
      property flake_attr : String

      @[JSON::Field(key: "instance-type")]
      property instance_type : String

      @[JSON::Field(key: "private-ip")]
      property private_ip : String

      @[JSON::Field(key: "public-ip")]
      property public_ip : String
    end

    class Roles
      include JSON::Serializable

      property client : Role
      property core : Role
    end

    class Role
      include JSON::Serializable

      property arn : String
    end
  end
end
