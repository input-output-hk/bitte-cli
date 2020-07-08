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
          ch_count += 1
          parallel_copy channel: ch,
            name: name,
            ip: instance.public_ip,
            flake: flake,
            flake_attr: instance.flake_attr,
            uid: instance.uid
          sleep(ch_count * 2) # this works around https://github.com/NixOS/nix/issues/3794
        end

        cluster.asgs.each do |name, asg|
          log.info { "Copying closures to ASG #{name}" }

          asg.instances.each do |instance|
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

      def parallel_copy(channel, name, ip, flake, flake_attr, uid)
        logger = log.for(name)

        spawn do
          begin
            logger.info { "Copying closure to #{name} (#{ip})" }
            sh! "nix", "copy",
              "--substitute-on-destination",
              "--to", "ssh://root@#{ip}",
              "#{flake}##{flake_attr}",
              log: logger

            logger.info { "Copied closure, starting nixos-rebuild ..." }

            sh! "nixos-rebuild", "switch",
              "--target-host", "root@#{ip}",
              "--flake", "#{flake}##{uid}",
              log: logger

            logger.info { "finished." }
          rescue ex
            log.error(exception: ex) { "failed copying to #{name} (#{ip})" }
          ensure
            channel.send nil
          end
        end
      end

      def set_ssh_config
        ENV["NIX_SSHOPTS"] ||= (SSH::COMMON_ARGS + ssh_key).join(" ")
      end

      # def cluster
      #   Cluster.new(
      #     profile: parent.flags.as(CLI::Flags).profile,
      #     flake: flake,
      #     name: cluster_name,
      #     region: parent.flags.as(CLI::Flags).region
      #   )
      # end

      def cluster
        @cluster ||= TerraformCluster.load
      end

      def cluster_name
        parent.flags.as(CLI::Flags).cluster
        :wa
      end

      def flake
        parent.flags.as(CLI::Flags).flake
      end
    end
  end
end
