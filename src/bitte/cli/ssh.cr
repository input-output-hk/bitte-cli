module Bitte
  class CLI
    register_sub_command ssh : SSH, description: "SSH to machines"

    class SSH < Admiral::Command
      include Helpers

      COMMON_ARGS = [
        "-C", # Requests compression of all data
        "-o", "NumberOfPasswordPrompts=0",
        "-o", "ServerAliveInterval=60",
        "-o", "ControlPersist=600",
        "-o", "StrictHostKeyChecking=accept-new",
      ]

      define_help description: "SSH to a machine"

      define_argument host

      define_flag term : String,
        default: ENV["TERM"]? || "xterm"

      property cluster : TerraformCluster?

      def run
        args = ssh_args
        log.debug { "ssh #{args.join(" ")}" }

        if @argv.empty?
          Process.exec("ssh", args: args, env: {"TERM" => "xterm"})
        else
          Process.exec("ssh", args: args, env: {"TERM" => "xterm"}, output: STDOUT)
        end
      end

      def ssh_args
        COMMON_ARGS + ssh_key + [
          "-x", # Disables X11 forwarding
          ( "-t" if @argv.empty? ), # force pseudo-tty
          "-p", "22",
          "root@#{ip}",
        ].compact + @argv.map{|a| Process.quote_posix(a.to_s) }
      end

      def ip
        if node = cluster.instances[arguments.host]
          node.public_ip
        else
          raise "No instance with name #{arguments.host} found"
        end
      end

      def cluster
        @cluster ||= TerraformCluster.load
      end
    end
  end
end
