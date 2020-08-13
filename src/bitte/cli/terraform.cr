module Bitte
  class CLI
    register_sub_command terraform : Terraform, description: "provisioning tool"

    class Terraform < Admiral::Command
      include Helpers

      define_help short: "h", description: "Create config.tf.json from the flake and apply it"

      define_argument realm : String, default: "core", required: true

      def run
        with_workspace "#{cluster}_#{realm}" do
          sh! "nix", "build",
            "#{flake}#clusters.#{cluster}.tf.#{realm}.output",
            "-o", "config.tf.json"

          sh! "terraform", "plan", "-out", plan_file

          wait_for_user

          sh! "terraform", "apply", plan_file
        end
      end

      def wait_for_user
        log.info { "applying the plan in 20 seconds!" }
        log.info { "Press return to apply immediately, Ctrl-C to cancel" }

        STDIN.read_timeout = 20
        STDIN.gets
      rescue IO::TimeoutError
      end

      def plan_file
        "#{cluster}.#{realm}.plan"
      end

      def realm : String
        arguments.realm
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
