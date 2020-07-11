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
        log.info { "Provisioning" }

        set_ssh_config

        # TODO: add the encrypt key here
        create_secrets
        wait_for_ssh(ip)
        copy_secrets
      end

      def create_secrets
        unless File.exists?("./secrets/consul.master.token.json")
          json = {
            acl:     {tokens: {master: UUID.random.to_s}},
            encrypt: `consul keygen`.strip,
          }.to_pretty_json
          raise "Couldn't create consul encrypt key" unless $?.success?
          File.write("./secrets/consul.master.token.json", json)
        end
      end

      def set_ssh_config
        ENV["NIX_SSHOPTS"] ||= (SSH::COMMON_ARGS + ssh_key).join(" ")
      end

      # TODO: replace with rsync
      def copy_secrets
        logger = Log.for(name)
        dst = "root@#{ip}"
        sh! "ssh", args: SSH::COMMON_ARGS + ssh_key + [dst, "mkdir", "-p", "/etc/consul.d"], logger: logger
        sh! "scp", (secrets/"consul.master.token.json").to_s, "#{dst}:/etc/consul.d/master-token.json", logger: logger
      end

      def skip?(name)
        if flags.only.any?
          !flags.only.includes?(name)
        else
          false
        end
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
