module Bitte
  class CLI
    register_sub_command terraform : Terraform, description: "provisining tool"

    class Terraform < Admiral::Command
      include Helpers

      define_help description: "Create the config.tf.json from the flake"

      def run
        sh! "nix", "build",
          "#{flake}#clusters.#{cluster}.terraform-output",
          "-o", "config.tf.json"
      end

      def flake
        parent.flags.as(CLI::Flags).flake
      end

      def cluster
        parent.flags.as(CLI::Flags).cluster
      end
    end
  end
end
