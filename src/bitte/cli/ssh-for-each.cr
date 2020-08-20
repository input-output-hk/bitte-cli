module Bitte
  class CLI
    register_sub_command pssh : SSHForEach, description: "Execute SSH command on all machines"

    class SSHForEach < Admiral::Command
      include Helpers

      define_help short: "h", description: "Execute command on all machines"

      define_flag term : String,
        default: ENV["TERM"]? || "xterm"

      property cluster : TerraformCluster?

      def run
        ch = Channel(Nil).new
        ch_count = 0

        cluster.instances.each do |name, instance|
          ch_count += 1
          parallel_ssh ch, name, instance.public_ip, @argv.map(&.to_s)
        end

        cluster.asgs.each do |name, asg|
          asg.instances.each do |instance|
            next unless ip = instance.public_ip
            ch_count += 1
            parallel_ssh ch, name, ip, @argv.map(&.to_s)
          end
        end

        ch_count.times do
          ch.receive
        end
      end

      def parallel_ssh(channel, name : String, ip : String, args, attempts = 10)
        host_ssh_args = SSH::COMMON_ARGS + ssh_key + ["root@#{ip}"] + args

        spawn do
          logger = Log.for(name)

          begin
            wait_for_ssh(ip)
            sh!("ssh", args: host_ssh_args, logger: logger)
          rescue ex
            if attempts > 0
              sleep rand(1..5)
              parallel_ssh(channel, name, ip, args, attempts - 1)
            else
              log.error(exception: ex) { "failed on #{name} (#{ip})" }
            end
          ensure
            channel.send nil
          end
        end
      end

      def cluster
        @cluster ||= TerraformCluster.load("core")
      end
    end
  end
end
