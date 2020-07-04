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

      def run
        ssh_args = COMMON_ARGS + [
          "-x", # Disables X11 forwarding
          "-t", # Force pseudo-terminal allocation
          "-p", "22",
          "root@#{ip}",
        ] + ssh_key
        log.debug { "ssh #{ssh_args.join(" ")}" }
        Process.exec("ssh", args: ssh_args, env: {"TERM" => "xterm"})
      end

      def ip
        if node = cluster[arguments.host]
          node.public_ip
        else
          raise "No instance with name #{arguments.host} found"
        end
      end

      def cluster
        Cluster.new(
          profile: parent.flags.as(CLI::Flags).profile,
          flake: parent.flags.as(CLI::Flags).flake,
          name: parent.flags.as(CLI::Flags).cluster,
          region: parent.flags.as(CLI::Flags).region
        )
      end
    end
  end
end
