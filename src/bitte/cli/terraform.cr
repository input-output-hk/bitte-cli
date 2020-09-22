module Bitte
  class CLI
    register_sub_command terraform : Terraform, description: "provisioning tool"
    register_sub_command tf : Terraform, description: "provisioning tool"

    class Terraform < Admiral::Command
      include Helpers

      define_help short: "h", description: "Create config.tf.json from the flake and apply it"

      define_argument workspace : String, required: true
    end
  end
end
