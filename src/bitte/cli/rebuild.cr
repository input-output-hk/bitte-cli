require "uri"

module Bitte
  class CLI
    register_sub_command rebuild : Rebuild, description: "nixos-rebuild the targets"

    class Rebuild < Admiral::Command
      include Helpers

      define_help short: "h", description: "nixos-rebuild"

      define_flag only : Array(String),
        description: "node names to include",
        default: Array(String).new

      define_flag delay : Int32,
        description: "seconds to delay between rebuilds",
        default: 0

      property cluster : TerraformCluster?

      def run
        set_ssh_config

        copy
        rebuild
      end

      def copy
        ch = Channel(Nil).new
        ch_count = 0

        cluster.instances.each do |name, instance|
          next if skip?(name)
          ch_count += 1
          wait_for_ssh(instance.public_ip)
          parallel_copy channel: ch,
            name: name,
            ip: instance.public_ip.not_nil!,
            flake_attr: instance.flake_attr,
            uid: instance.uid
        end

        if asgs = cluster.asgs
          asgs.each do |_, asg|
            asg.instances.each do |instance|
              next if skip?(instance.name)
              ch_count += 1
              parallel_copy channel: ch,
                name: instance.name,
                ip: instance.public_ip.not_nil!,
                flake_attr: asg.flake_attr,
                uid: asg.uid
            end
          end
        end

        ch_count.times do
          ch.receive
        end
      end

      def rebuild
        tasks = [] of Proc(Nil)
        pending = [] of String

        cluster.instances.each do |name, instance|
          next if skip?(name)
          pending << name
          tasks << ->() {
            rebuild name: name,
              ip: instance.public_ip.not_nil!,
              flake_attr: instance.flake_attr,
              uid: instance.uid
          }
        end

        if asgs = cluster.asgs
          asgs.each do |_, asg|
            asg.instances.each do |instance|
              next if skip?(instance.name)
              pending << instance.name
              tasks << ->(){
                rebuild name: instance.name,
                  ip: instance.public_ip.not_nil!,
                  flake_attr: asg.flake_attr,
                  uid: asg.uid
              }
            end
          end
        end

        log.info { "rebuilding #{pending.join(" ")}" }

        tasks.each do |task|
          task.call
          if delay > 0.seconds && task != tasks.last
            log.debug { "waiting #{delay}..." }
            sleep delay
          end
        end
      end

      def skip?(name)
        if flags.only.any?
          !flags.only.includes?(name)
        else
          false
        end
      end

      def delay
        flags.delay.seconds
      end

      def parallel_copy(channel : Channel(Nil), name : String, ip : String, flake_attr : String, uid : String, attempts = 10)
        spawn do
          begin
            parallel_copy(name, ip, flake_attr, uid, attempts)
          ensure
            channel.send nil
          end
        end
      end

      def parallel_copy(name : String, ip : String, flake_attr : String, uid : String, attempts = 10)
        logger = log.for(name)
        logger.info { "Copying closure to #{cluster.s3_cache}" }

        ENV["IP"] = ip
        sh! "nix", "run", "#{flake}#nixosConfigurations.#{uid}.config.secrets.generateScript"

        sh! "nix", "build", "#{flake}##{flake_attr}", logger: logger

        sh! "nix", "copy",
          "--to", "#{cluster.s3_cache}&secret-key=secrets/nix-secret-key-file",
          "#{flake}##{flake_attr}",
          logger: logger

        logger.info { "Copying closure to #{name} (#{ip})" }

        sh! "nix", "copy",
          "--substitute-on-destination",
          "--to", "ssh://root@#{ip}",
          "#{flake}##{flake_attr}",
          logger: logger

        logger.info { "Copied closure" }
      rescue ex
        if attempts > 0
          sleep [2, 3, 5, 7, 11].sample.seconds
          parallel_copy(name, ip, flake_attr, uid, attempts - 1)
        else
          log.error(exception: ex) { "failed copying to #{name} (#{ip})" }
        end
      end

      def rebuild(name : String, ip : String, flake_attr : String, uid : String, attempts = 10) : Nil
        logger = log.for(name)
        logger.info { "nixos-rebuild for #{flake}##{uid}" }

        sh! "nixos-rebuild", "switch",
          "--target-host", "root@#{ip}",
          "--flake", "#{flake}##{uid}",
          logger: logger

        logger.info { "finished." }
      rescue ex
        if attempts > 0
          sleep [2, 3, 5, 7, 11].sample.seconds
          rebuild(name, ip, flake_attr, uid, attempts - 1)
        else
          log.error(exception: ex) { "failed copying to #{name} (#{ip})" }
        end
      end

      def set_ssh_config
        ENV["NIX_SSHOPTS"] ||= (SSH.common_args + ssh_key).join(" ")
      end

      def cluster
        @cluster ||= TerraformCluster.load("clients")
      end

      def flake
        "."
      end
    end
  end
end
