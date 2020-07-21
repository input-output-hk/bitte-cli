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

        sh! "terraform", "plan", "-out", "#{cluster}.plan"

        log.info { "applying the plan in 20 seconds!" }
        log.info { "Press return to apply immediately, Ctrl-C to cancel" }

        STDIN.read_timeout = 20
        begin
          STDIN.gets
        rescue IO::TimeoutError
        end

        sh! "terraform", "apply", "#{cluster}.plan"
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
