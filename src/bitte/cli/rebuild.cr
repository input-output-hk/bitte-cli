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
            ip: instance.public_ip.not_nil!,
            flake: flake,
            flake_attr: instance.flake_attr,
            uid: instance.uid
          sleep(ch_count * 2) # this works around https://github.com/NixOS/nix/issues/3794
        end

        cluster.asgs.each do |name, asg|
          asg.instances.each do |instance|
            next if skip?(name)
            ch_count += 1
            parallel_copy channel: ch,
              name: instance.name,
              ip: instance.public_ip.not_nil!,
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

      def parallel_copy(channel : Channel(Nil), name : String, ip : String, flake : String, flake_attr : String, uid : String, attempts = 10)
        spawn do
          begin
            parallel_copy(name, ip, flake, flake_attr, uid, attempts)
          ensure
            channel.send nil
          end
        end
      end

      def parallel_copy(name : String, ip : String, flake : String, flake_attr : String, uid : String, attempts = 10)
        logger = log.for(name)
        logger.info { "Copying closure to #{name} (#{ip})" }

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
          sleep [2, 3, 5, 7, 11].sample.seconds
          parallel_copy(name, ip, flake, flake_attr, uid, attempts - 1)
        else
          log.error(exception: ex) { "failed copying to #{name} (#{ip})" }
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
