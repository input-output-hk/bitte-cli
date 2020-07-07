module Bitte
  class CLI
    register_sub_command rebuild : Rebuild, description: "nixos-rebuild the targets"

    class Rebuild < Admiral::Command
      include Helpers

      define_help description: "nixos-rebuild"

      def run
        set_ssh_config

        ch = Channel(Nil).new

        nodes = cluster.nodes.values + cluster.asg_nodes

        nodes.each do |node|
          sh! "nix", "copy",
            "--substitute-on-destination",
            "--to", "ssh://root@#{node.public_ip}",
            "#{flake}#nixosConfigurations.#{node.uid}.config.system.build.toplevel"
        end

        nodes.each do |node|
          spawn do
            begin
              sh! "nixos-rebuild",
                "--flake", "#{flake}##{node.uid}",
                "switch", "--target-host", "root@#{node.public_ip}"
            rescue ex
              log.error(exception: ex) { "nixos-rebuild failed"}
            ensure
              ch.send nil
            end
          end
        end

        nodes.each do |_|
          ch.receive
        end
      end

      def set_ssh_config
        ENV["NIX_SSHOPTS"] ||= (SSH::COMMON_ARGS + ssh_key).join(" ")
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
