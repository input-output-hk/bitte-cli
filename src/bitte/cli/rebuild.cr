module Bitte
  class CLI
    register_sub_command rebuild : Rebuild, description: "nixos-rebuild the targets"

    class Rebuild < Admiral::Command
      include Helpers

      define_help description: "nixos-rebuild"

      def run
        ch = Channel(Nil).new

        cluster.nodes.each do |name, node|
          sh! "nix", "copy",
            "--substitute-on-destination",
            "--to", "ssh://root@#{node.public_ip}",
            "#{flake}#nixosConfigurations.#{cluster_name}-#{name}.config.system.build.toplevel"
        end

        cluster.nodes.each do |name, node|
          spawn do
            begin
              sh! "nixos-rebuild",
                "--flake", "#{flake}##{cluster_name}-#{name}",
                "switch", "--target-host", "root@#{node.public_ip}"
            rescue ex
              log.error(exception: ex) { "nixos-rebuild failed"}
            ensure
              ch.send nil
            end
          end
        end

        cluster.nodes.each do |_, _|
          ch.receive
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
