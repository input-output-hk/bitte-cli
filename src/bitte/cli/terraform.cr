module Bitte
  class CLI
    register_sub_command terraform : Terraform, description: "provisioning tool"
    register_sub_command tf : Terraform, description: "provisioning tool"
  end
end
