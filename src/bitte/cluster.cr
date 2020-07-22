module Bitte
  class TerraformCluster
    include JSON::Serializable

    extend CLI::Helpers

    def self.load
      mem = IO::Memory.new
      sh!("terraform", args: ["output", "-json", "cluster"], output: mem)
      from_json(mem.to_s).tap do |cluster|
        cluster.asgs.each do |name, asg|
          asg.cluster = cluster
          asg.name = name
        end
      end
    end

    property asgs : Hash(String, ASG)
    property flake : String
    property instances : Hash(String, Instance)
    property kms : String
    property name : String
    property nix : String
    property region : String
    property roles : Roles

    class ASG
      include JSON::Serializable

      property name : String?
      property cluster : TerraformCluster?
      property flake_attr : String
      property instance_type : String
      property uid : String
      property arn : String

      record Instance,
         asg : ASG,
         name : String,
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
        AWS::Client.new
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

      property flake_attr : String
      property instance_type : String
      property name : String
      property private_ip : String
      property public_ip : String
      property tags : Hash(String, String)
      property uid : String
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
