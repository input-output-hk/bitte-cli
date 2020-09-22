module Bitte
  class CLI
    register_sub_command terraform : Terraform, description: "provisioning tool"

    class Terraform < Admiral::Command
      include Helpers

      define_help short: "h", description: "Create config.tf.json from the flake"

      define_argument workspace : String, required: true

      register_sub_command plan : Plan, description: "plan"

      class Plan < Admiral::Command
        include Helpers

        define_help short: "h", description: "Create a terraform plan from the config"

        def run
          with_workspace(cluster, workspace) do
            sh! "terraform", "plan"
          end
        end

        def workspace : String
          parent.arguments.as(CLI::Terraform::Arguments).workspace
        end

        def cluster
          parent.parent.flags.as(CLI::Flags).cluster
        end
      end

      class Apply < Admiral::Command
        include Helpers

        define_help short: "h", description: "Create a terraform plan from the config"

        def run
          with_workspace(cluster, workspace) do
            sh! "terraform", "apply", "#{workspace}.plan"
            FileUtils.rm_rf("#{workspace}.plan")
          end
        end

        def workspace : String
          parent.arguments.as(CLI::Terraform::Arguments).workspace
        end

        def cluster
          parent.parent.flags.as(CLI::Flags).cluster
        end
      end
    end
  end
end
