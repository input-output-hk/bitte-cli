require "uri"

module Bitte
  class CLI
    register_sub_command rebuild : Rebuild, description: "nixos-rebuild the targets"

    class Rebuild < Admiral::Command
      include Helpers

      define_help description: "nixos-rebuild"

      define_flag only : Array(String),
        description: "node names to include",
        default: Array(String).new

      define_flag dirty : Bool,
        description: "Use current directory as flake",
        default: false

      property cluster : TerraformCluster?

      def run
        set_ssh_config

        flake = flags.dirty ? "." : cluster.flake

        ch = Channel(Nil).new
        ch_count = 0

        cluster.instances.each do |name, instance|
          next if skip?(name)
          ch_count += 1
          wait_for_ssh(instance.public_ip)
          parallel_copy channel: ch,
            name: name,
            ip: instance.public_ip,
            flake: flake,
            flake_attr: instance.flake_attr,
            uid: instance.uid
          sleep(ch_count * 2) # this works around https://github.com/NixOS/nix/issues/3794
        end
#
        cluster.asgs.each do |name, asg|
          asg.instances.each do |instance|
            next if skip?(name)
            ch_count += 1
            parallel_copy channel: ch,
              name: instance.name,
              ip: instance.public_ip,
              flake: flake,
              flake_attr: asg.flake_attr,
              uid: asg.uid
            sleep(ch_count * 2) # this works around https://github.com/NixOS/nix/issues/3794
          end
        end

        ch_count.times do
          ch.receive
        end
      end

      def skip?(name)
        if flags.only.any?
          !flags.only.includes?(name)
        else
          false
        end
      end

      def substituters
        [
          "https://cache.nixos.org",
          "https://hydra.iohk.io",
          "https://manveru.cachix.org",
        ]
      end

      def trusted_public_keys
        [
          "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=",
          "manveru.cachix.org-1:L5nJHSinfA2K5dDCG3KAEadwf/e3qqhuBr7yCwSksXo=",
          "hydra.iohk.io:f/Ea+s+dFdN+3Y/G+FDgSq+a5NEWhJGzdjvKNGv0/EQ=",
        ]
      end

      def parallel_copy(channel, name, ip, flake, flake_attr, uid, attempts = 10)
        logger = log.for(name)

        spawn do
          begin
            logger.info { "Copying closure to #{name} (#{ip})" }

            # sh! "nix", "copy",
            #   "--substitute-on-destination",
            #   "--to", "ssh://root@#{ip}",
            #   cluster.nix
            #
            # sh! "rsync", args: [
            #   "-e", (["ssh"] + SSH::COMMON_ARGS + ssh_key ).join(" "),
            #   "-r", "#{flake}/", "root@#{ip}:source/",
            # ]
            #
            # sh! "ssh", args: SSH::COMMON_ARGS + ssh_key + [
            #   "root@#{ip}",
            #   [
            #     "#{cluster.nix}/bin/nix",
            #     "--experimental-features",
            #     "'flakes nix-command'",
            #     "shell", "nixpkgs#git", "-c",
            #     "#{cluster.nix}/bin/nix",
            #     "--experimental-features",
            #     "'flakes nix-command'",
            #     "build",
            #     "--substituters", "'#{ substituters.join(" ") }'",
            #     "--trusted-public-keys", "'#{trusted_public_keys.join(" ")}'",
            #     "'./source##{flake_attr}'"
            #   ].join(" ")
            # ]

            sh! "nix", "copy",
              "--substitute-on-destination",
              "--to", "ssh://root@#{ip}",
              "#{flake}##{flake_attr}",
              logger: logger

            logger.info { "Copied closure, starting nixos-rebuild ..." }

            sh! "nixos-rebuild", "switch",
              "--target-host", "root@#{ip}",
              "--flake", "#{flake}##{uid}",
              logger: logger

            logger.info { "finished." }
          rescue ex
            if attempts > 0
              sleep rand(1..5)
              parallel_copy(channel, name, ip, flake, flake_attr, uid, attempts - 1)
            else
              log.error(exception: ex) { "failed copying to #{name} (#{ip})" }
            end
          ensure
            channel.send nil
          end
        end
      end

      def set_ssh_config
        ENV["NIX_SSHOPTS"] ||= (SSH::COMMON_ARGS + ssh_key).join(" ")
      end

      def cluster
        @cluster ||= TerraformCluster.load
      end

      def flake
        parent.flags.as(CLI::Flags).flake
      end
    end
  end
end
