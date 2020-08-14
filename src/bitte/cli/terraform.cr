module Bitte
  class CLI
    register_sub_command terraform : Terraform, description: "provisioning tool"

    class Terraform < Admiral::Command
      include Helpers

      define_help short: "h", description: "Create config.tf.json from the flake and apply it"

      define_argument realm : String, required: true

      def run
        sh! "nix", "build",
          "#{flake}#clusters.#{cluster}.tf.#{realm}.output",
          "-o", "config.tf.json.ln"

        FileUtils.rm_rf("config.tf.json")
        FileUtils.cp(File.readlink("config.tf.json.ln"), "config.tf.json")

        with_workspace realm do
          sh! "terraform", "apply"
        end
      ensure
        FileUtils.rm_rf("config.tf.json.ln")
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

      def cluster_name
        cluster
      end
    end
  end
end
