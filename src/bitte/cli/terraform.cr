module Bitte
  class CLI
    register_sub_command terraform : Terraform, description: "provisioning tool"

    class Terraform < Admiral::Command
      include Helpers

      define_help short: "h", description: "Create config.tf.json from the flake and apply it"

      define_argument realm : String, default: "core", required: true

      def run
        with_workspace do
          sh! "nix", "build",
            "#{flake}#clusters.#{cluster}.tf.#{realm}.output",
            "-o", "config.tf.json"

          sh! "terraform", "plan", "-out", plan_file

          wait_for_user

          sh! "terraform", "apply", plan_file
        end
      end

      def with_workspace
        original = tf_workspace_show

        if original == workspace
          return yield
        end

        available = tf_workspace_list

        if available.includes?(workspace)
          tf_workspace_select workspace
        else
          tf_workspace_new
        end

        yield
      ensure
        tf_workspace_select original if original
      end

      def tf_workspace_select(name) : Nil
        sh! "terraform", "workspace", "select", name
      end

      def tf_workspace_new : Nil
        sh! "terraform", "workspace", "new", workspace
      end

      def tf_workspace_show : String
        output = IO::Memory.new
        sh! "terraform", "workspace", "show", output: output
        output.to_s.strip
      end

      def tf_workspace_list : Array(String)
        output = IO::Memory.new
        sh! "terraform", "workspace", "list", output: output
        output.to_s.split - ["*"]
      end

      def wait_for_user
        log.info { "applying the plan in 20 seconds!" }
        log.info { "Press return to apply immediately, Ctrl-C to cancel" }

        STDIN.read_timeout = 20
        STDIN.gets
      rescue IO::TimeoutError
      end

      def workspace
        "#{cluster}.#{realm}"
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
