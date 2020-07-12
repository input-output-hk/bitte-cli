require "uuid"

module Bitte
  class CLI
    register_sub_command provision : Provision, description: "Provision a deployment"

    class Provision < Admiral::Command
      include Helpers

      define_help description: "Initial provisioning from Terraform!"

      define_flag ip : String, description: "ip of the node", required: true
      define_flag name : String, description: "name of the node", required: true
      define_flag cluster : String, description: "name of the cluster", required: true

      def run
        logger = Log.for(name)
        logger.info { "Waiting for user_data to be applied..." }

        set_ssh_config
        wait_for_ssh(ip)

        ready = false

        until ready
          sleep 5

          begin
            output = IO::Memory.new
            sh! "ssh", args: SSH::COMMON_ARGS + ssh_key + ["root@#{ip}", "cat", "/etc/ready"], logger: logger, output: output
            ready = output.to_s =~ /true/
          rescue RetryableError
          end
        end

        logger.warn { "Ready to deploy. Don't forget to copy ACME certs first if you have a backup!" }
      end

      def set_ssh_config
        ENV["NIX_SSHOPTS"] ||= (SSH::COMMON_ARGS + ssh_key).join(" ")
      end

      def ip
        flags.ip
      end

      def name
        flags.name
      end

      def cluster_name
        flags.cluster
      end
    end
  end
end
