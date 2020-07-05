module Bitte
  class CLI
    register_sub_command scp : SCP, description: "SCP files & folders to or from nodes"

    class SCP < Admiral::Command
      define_help description: "SCP to/from nodes"

      register_sub_command to : To, description: "SCP files & folders to nodes"

      class To < Admiral::Command
        include Helpers
        define_help description: "SCP files & folders to a node"

        define_argument host

        define_flag term : String,
          default: ENV["TERM"]? || "xterm"

        def run
          src = @argv.shift
          dst = @argv.shift
          scp_args = SSH::COMMON_ARGS + [
            src.to_s, "root@#{ip}:#{dst}",
          ]
          sh! "scp", args: scp_args
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

      register_sub_command from : From, description: "SCP files & folders from nodes"

      class From < Admiral::Command
        include Helpers
        define_help description: "SCP files & folders from a node"

        define_argument host

        define_flag term : String,
          default: ENV["TERM"]? || "xterm"

        def run
          src = @argv.shift
          dst = @argv.shift
          scp_args = SSH::COMMON_ARGS + [
            "root@#{ip}:#{src}", dst.to_s,
          ]
          sh! "scp", args: scp_args
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
end
