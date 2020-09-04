module Bitte
  class CLI
    register_sub_command ssh : SSH, description: "SSH to machines"

    class SSH < Admiral::Command
      include Helpers

      def self.common_args(strict_host_key_checking = "accept-new")
        [
          "-C", # Requests compression of all data
          "-o", "NumberOfPasswordPrompts=0",
          "-o", "ServerAliveInterval=60",
          "-o", "ControlPersist=600",
          "-o", "StrictHostKeyChecking=#{strict_host_key_checking}",
        ]
      end

      class Runner
        include Helpers

        property host : String, term : String, given_args : Array(String), cluster : TerraformCluster

        def initialize(@host, @term, @given_args)
          @cluster = TerraformCluster.load("core")
        end

        def exec
          args = ssh_args
          log.debug { "ssh #{args.join(" ")}" }

          if @given_args.empty?
            Process.exec("ssh", args: args, env: {"TERM" => term})
          else
            Process.exec("ssh", args: args, env: {"TERM" => term}, output: STDOUT)
          end
        end

        def run(output = STDOUT)
          args = ssh_args
          log.debug { "ssh #{args.join(" ")}" }

          if @given_args.empty?
            Process.run("ssh", args: args, env: {"TERM" => term})
          else
            Process.run("ssh", args: args, env: {"TERM" => term}, output: output)
          end
        end

        def ssh_args
          SSH.common_args + ssh_key + [
            "-x",                         # Disables X11 forwarding
            ("-t" if @given_args.empty?), # force pseudo-tty
            "-p", "22",
            "root@#{ip}",
          ].compact + @given_args
        end

        def ip
          if node = cluster.instances[@host]
            node.public_ip
          else
            raise "No instance with name #{@host} found"
          end
        end
      end

      define_help short: "h", description: "SSH to a machine"

      define_argument host

      define_flag term : String, default: "xterm"

      def run
        if host = arguments.host
          Runner.new(host, flags.term, @argv.map { |a| Process.quote_posix(a.to_s) }).exec
        else
          raise "host must be given"
        end
      end
    end
  end
end
