require "uuid"

module Bitte
  class CLI
    register_sub_command deploy : Deploy, description: "initiate a deployment"

    class Deploy < Admiral::Command
      include Helpers

      define_help description: "Deploy all the things!"

      def run
        log.info { "Copying " }

        create_secrets
        copy_nix
        rebuild
      end

      def rebuild
        cluster.nodes.each do |name, node|
          sh! "nixos-rebuild",
            "--flake", "#{flake}##{cluster_name}-#{name}",
            "switch", "--target-host", "root@#{node.public_ip}"
        end
      end

      def create_secrets
        unless File.exists?("./secrets/consul.master.token.json")
          json = {acl: {tokens: {master: UUID.random.to_s}}}.to_pretty_json
          File.write("./secrets/consul.master.token.json", json)
        end

        sh! "bitte-certs", cluster_name
      end

      def copy_nix
        cluster.nodes.each do |name, node|
          sh! "nix", "copy",
            "--substitute-on-destination",
            "--to", "ssh://root@#{node.public_ip}",
            "#{flake}#clusters.#{cluster_name}.#{name}.config.system.build.toplevel"

          copy_secrets(node)
        end
      end

      def copy_secrets(node)
        dst = "root@#{node.public_ip}"
        sh! "ssh", dst, "mkdir", "-p", "/etc/consul.d"
        sh! "scp", "./secrets/consul.master.token.json", "#{dst}:/etc/consul.d/master-token.json"
        sh! "scp", "./secrets/#{cluster_name}.pem", "#{dst}:/run/keys/ca.pem"
        sh! "scp", "./encrypted/#{cluster_name}/#{node.name}.enc.json", "#{dst}:/run/keys/certs.enc.json"
        Dir.glob("encrypted/#{cluster_name}/*.pem") do |pem|
          sh! "scp", pem, "#{dst}:/run/keys/#{File.basename(pem)}"
        end
      end

      def cluster
        Cluster.new(
          profile: parent.flags.as(CLI::Flags).profile,
          flake: flake,
          name: cluster_name,
          region: parent.flags.as(CLI::Flags).region
        )
      end

      def cluster_name
        parent.flags.as(CLI::Flags).cluster
      end

      def flake
        parent.flags.as(CLI::Flags).flake
      end
    end
  end
end
