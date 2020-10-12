module Bitte
  class CLI
    register_sub_command terraform : Terraform, description: "provisioning tool"
    register_sub_command tf : Terraform, description: "provisioning tool"

    class Terraform < Admiral::Command
      include Helpers

      define_help short: "h", description: "Create config.tf.json from the flake and apply it"

      define_argument workspace : String, required: true

      def run
        args = ["run", ".#clusters.#{cluster}.tf.#{workspace}.terraform"] + @argv.map(&.to_s)
        sh! "nix", args: args, output: STDOUT
      end

      def workspace : String
        arguments.workspace
      end

      def flake
        parent.flags.as(CLI::Flags).flake
      end

      def cluster
        parent.flags.as(CLI::Flags).cluster
      end

      def cluster_name
        cluster
      end
    end
  end
end
