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
        set_ssh_config
        copy_nix
        rebuild
      end

      def create_secrets
        unless File.exists?("./secrets/consul.master.token.json")
          json = {acl: {tokens: {master: UUID.random.to_s}}}.to_pretty_json
          File.write("./secrets/consul.master.token.json", json)
        end
      end

      def set_ssh_config
        ENV["NIX_SSHOPTS"] ||= (SSH::COMMON_ARGS + ssh_key).join(" ")
      end

      def copy_nix
        cluster.nodes.each do |name, node|
          log.info { "Copying Nix to #{name} ..." }
          # File.open(temp, "w+") do |nix_conf|
          #   sh! "nix", output: nix_conf,
          #     args: ["eval", "--raw",
          #            "--apply", "builtins.readFile",
          #            %(#{base}.environment.etc."nix/nix.conf".source)]
          # end

          flake_source = IO::Memory.new
          sh! "nix", output: flake_source, args: [
            "eval", "--raw", "#{flake}#self.outPath"
          ]

          sh! "rsync", args: [
            "-e", (["ssh"] + SSH::COMMON_ARGS + ssh_key ).join(" "),
            "-r", "#{ flake_source }/", "root@#{node.public_ip}:source/",
          ]

          nix_flakes = IO::Memory.new
          sh! "nix", output: nix_flakes, args: [
            "eval", "--raw", "#{flake}#nixFlakes.outPath"
          ]

          sh! "nix", "copy",
            "--substitute-on-destination",
            "--to", "ssh://root@#{node.public_ip}",
            "#{flake}#nixFlakes"

          sh! "ssh", args: SSH::COMMON_ARGS + ssh_key + [
            "root@#{node.public_ip}",
            "#{nix_flakes}/bin/nix build --experimental-features 'flakes nix-command' './source#nixosConfigurations.#{cluster_name}-#{name}.config.system.build.toplevel'"
          ]

          sh! "nix", "copy",
            "--substitute-on-destination",
            "--to", "ssh://root@#{node.public_ip}",
            "#{flake}#nixosConfigurations.#{cluster_name}-#{name}.config.system.build.toplevel"

          copy_secrets(node)
        end
      end

      def rebuild
        cluster.nodes.each do |name, node|
          sh! "nixos-rebuild",
            "--flake", "#{flake}##{cluster_name}-#{name}",
            "switch", "--target-host", "root@#{node.public_ip}"
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
