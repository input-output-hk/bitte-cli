require "tablo"

module Bitte
  class CLI
    register_sub_command info : Info, description: "Show information about clusters and instances"

    class Info < Admiral::Command
      include Helpers

      define_help description: "Show information about clusters and instances"

      def run
        # if cluster_name.empty?
        #   clusters.each do |cluster|
        #     puts cluster.name
        #   end
        # else
        #   cluster.nodes.each do |node|
        #     puts node.name
        #   end
        # end
      end

      def clusters
        nix_eval "#{flake}#clusters", "builtins.attrNames" do |output|
          Array(String).from_json(output.to_s).map do |name|
            Cluster.new(flake: flake, name: name)
          end
        end
      end

      def cluster
        cluster = Cluster.new(flake: flake, name: cluster_name)
        nix_eval "#{flake}#clusters.#{cluster_name}.nodes", "builtins.attrNames" do |output|
          Array(String).from_json(output.to_s).map do |name|
            cluster.node_names << name
          end
        end
        cluster
      end

      def nix_eval(attr, apply)
        sh! "nix", "eval", "--json", attr, "--apply", apply do |output|
          yield output
        end
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
