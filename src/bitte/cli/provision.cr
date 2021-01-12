require "uuid"

module Bitte
  class CLI
    register_sub_command provision : Provision, description: "Provision a deployment"

    class Provision < Admiral::Command
      include Helpers

      define_help short: "h", description: "Initial provisioning from Terraform!"

      define_flag ip : String, description: "ip of the node", required: true
      define_flag name : String, description: "name of the node", required: true
      define_flag cluster : String, description: "name of the cluster", required: true
      define_flag flake : String, description: "flake location", required: true
      define_flag attr : String, description: "flake host attr", required: true
      define_flag cache : String, description: "push to this cache first", required: true

      def run
        logger = Log.for(name)
        logger.info { "Waiting for user_data to be applied..." }

        set_ssh_config
        wait_for_ssh(ip)

        sh! "ssh", args: temp_ssh_args + [
          "root@#{ip}", "until grep true /etc/ready &>/dev/null; do sleep 1; done",
        ], logger: logger

        logger.warn { "Ready to deploy." }

        sh! "ssh-keygen", "-R", ip
        sh! "nix", "copy",
          "--to", cache,
          "#{flake}#nixosConfigurations.#{flake_attr}.config.system.build.toplevel",
          logger: logger

        sh! "nix", "copy",
          "--substitute-on-destination",
          "--to", "ssh://root@#{ip}",
          "#{flake}#nixosConfigurations.#{flake_attr}.config.system.build.toplevel",
          logger: logger

        logger.info { "Copied closure, starting nixos-rebuild ..." }

        sh! "nixos-rebuild", "switch",
          "--target-host", "root@#{ip}",
          "--flake", "#{flake}##{flake_attr}",
          logger: logger
      end

      def ssh(*cmds)
        sh! "ssh", args: temp_ssh_args + [
          "root@#{ip}",
        ] + cmds.to_a, logger: logger, output: output
      end

      def temp_ssh_args
        SSH.common_args(strict_host_key_checking: "no") + ssh_key
      end

      def set_ssh_config
        ENV["NIX_SSHOPTS"] ||= temp_ssh_args.join(" ")
      end

      def cache
        "#{flags.cache}&secret-key=secrets/nix-secret-key-file"
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

      def flake
        flags.flake
      end

      def flake_attr
        flags.attr
      end
    end
  end
end
