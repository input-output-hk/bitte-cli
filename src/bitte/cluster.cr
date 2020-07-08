module Bitte
  class TerraformCluster
    include JSON::Serializable

    extend CLI::Helpers

    def self.load
      mem = IO::Memory.new
      sh!("terraform", args: ["output", "-json", "cluster"], output: mem)
      from_json(mem.to_s)
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

      property flake_attr : String
      property instance_type : String
      property uid : String
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

  class Cluster
    include CLI::Helpers

    property flake : String
    property name : String
    property region : String
    property profile : String
    property nodes = Hash(String, Node).new
    getter hydrated = false

    def initialize(@profile, @region, @flake, @name)
      populate
      hydrate
    end

    def asg_nodes
      aws_asgs.flat_map { |asg|
        instances = aws_client.describe_instances(
          asg.instances.map(&.instance_id)
        ).reservations.map(&.instances).flatten

        asg.instances.map do |asgi|
          i = instances.find { |instance| instance.instance_id == asgi.instance_id }
          next unless i

          tags = i.tags_hash
          if i && tags["Cluster"]? == self.name
            Node.new(
              cluster: self,
              name: asgi.instance_id,
              private_ip: i.private_ip_address.not_nil!,
              public_ip: i.public_ip_address,
            ).tap { |node| node.tags = tags }
          else
            raise "Can't find #{asgi.instance_id}"
          end
        end
      }.compact
    end

    def populate
      nix_eval "#{flake}#clusters.#{name}.topology.nodes" do |output|
        Hash(String, TopologyNode).from_json(output.to_s).each do |name, node|
          nodes[name] = Node.new(
            cluster: self,
            name: name,
            private_ip: node.private_ip
          )
        end
      end
    end

    def hydrate
      return if hydrated

      mem = IO::Memory.new
      sh!("terraform", args: ["output", "instances"], output: mem)
      ips = Hash(String, String).from_json(mem.to_s)
      ips.each do |node_name, ip|
        @nodes[node_name].public_ip = ip
      end

      aws_instances.each do |instance|
        tags = instance.tags_hash

        next if tags["aws:autoscaling:groupName"]?
        next unless tags["Cluster"]? == name
        next unless node_name = tags["Name"]?

        @nodes[node_name].tags = tags
      end

      hydrated = true
    end

    def [](node_name)
      nodes[node_name]
    end

    def aws_asgs
      aws_client.auto_scaling_groups.auto_scaling_groups
    end

    def aws_instances
      aws_client.describe_instances.reservations.map(&.instances).flatten
    end

    def aws_client
      AWS::Client.new(region: region, profile: profile)
    end

    class Node
      include CLI::Helpers

      property cluster : Cluster
      property name : String
      property public_ip : String?
      property private_ip : String
      property tags = Hash(String, String).new

      def initialize(@cluster, @name, @private_ip, @public_ip = nil)
      end

      def uid
        tags["UID"]
      end

      def region
        cluster.region
      end
    end
  end
end
