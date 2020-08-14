require "http/client"
require "../terraform"
require "socket"

module Bitte
  class CLI
    module Helpers
      def sh!(cmd : String,
              args : Array(String)?,
              env : Process::Env = nil,
              input : Process::Stdio = Process::Redirect::Pipe,
              output : IO? = nil,
              error : IO? = nil,
              logger : Log? = self.log)
        logger.debug { "run: #{cmd} #{args.join(" ")}" }

        Process.run(
          cmd,
          args: args,
          env: env,
          input: input,
          output: output || LogIO.new(logger),
          error: error || LogIO.new(logger)
        ) do |process|
          yield process
        end

        raise RetryableError.new("Process exited with #{$?.exit_status}") unless $?.success?
      end

      def sh!(cmd : String,
              args : Enumerable(String)?,
              env : Process::Env = nil,
              input : Process::Stdio = Process::Redirect::Close,
              output : IO? = nil,
              error : IO? = nil,
              logger : Log? = self.log)
        sh!(cmd, args: args, env: env, input: input, output: output, error: error, logger: logger) { |_| }
      end

      def sh!(cmd : String,
              *args : String,
              env : Process::Env = nil,
              input : Process::Stdio = Process::Redirect::Close,
              output : IO? = nil,
              error : IO? = nil,
              logger : Log? = self.log)
        sh!(cmd, args: args.to_a, env: env, input: input, output: output, error: error, logger: logger) { |_| }
      end

      def sh!(cmd : String,
              *args : String,
              env : Process::Env = nil,
              input : Process::Stdio = Process::Redirect::Close,
              output : IO = LogIO.new(log),
              error : IO = LogIO.new(log))
        sh!(cmd, args: args.to_a, env: env, input: input, output: output, error: error) { |process| yield process }
      end

      def ssh_key
        path = secrets/"ssh-#{cluster_name}"
        if File.exists?(path.to_s)
          ["-i", path.to_s]
        else
          [] of String
        end
      end

      def cluster_name
        cluster.name
      end

      def secrets
        Path["secrets"]
      end

      def encrypted
        Path["encrypted"]
      end

      def mtime(path)
        File.info(path.to_s).modification_time
      end

      def log
        Log.for(log_name)
      end

      def log_name
        self.class.to_s
      end

      def nix_eval(attr, apply)
        output = IO::Memory.new
        sh! "nix", "eval", "--json", attr, "--apply", apply
        yield output
      end

      def nix_eval(attr)
        output = IO::Memory.new
        sh! "nix", "eval", "--json", attr, output: output
        yield output
      end

      # We wait ~2 minutes for a connection
      def wait_for_ssh(ip)
        Log.debug { "Connecting to #{ip}:22..." }

        120.downto(0) do |i|
          begin
            TCPSocket.open(ip, 22) do
              log.debug { "Connected to #{ip}." }
            end

            return
          rescue Socket::ConnectError
            log.debug { "Connection to #{ip} failed again. #{i} attempts remaining" }
            sleep 1
          end
        end

        raise "Couldn't connect to #{ip}"
      end

      def with_workspace(name)
        original = tf_workspace_show

        if original != name
          tf_workspace_new(name) unless tf_workspace_list.includes?(name)
          tf_workspace_select name
        end

        yield
      ensure
        tf_workspace_select original if original && original != name
      end

      def tf_workspace_select(name) : Nil
        sh! "terraform", "workspace", "select", name
        sh! "terraform", "init"
      end

      def tf_workspace_new(name) : Nil
        Bitte::Terraform.create_workspace(tf_organization, name)
      end

      def tf_workspace_show : String
        output = IO::Memory.new
        sh! "terraform", "workspace", "show", output: output
        output.to_s.strip
      end

      def tf_workspace_list : Array(String)
        Bitte::Terraform.list_workspaces(tf_organization).map(&.attributes.name)
      end

      def tf_organization
        ENV["TERRAFORM_ORGANIZATION"]? ||
          begin
            state = NamedTuple(
              backend: NamedTuple(
                config: NamedTuple(
                  organization: String))).from_json(
              File.read(".terraform/terraform.tfstate"))

            state[:backend][:config][:organization]
          end
      end
    end
  end
end
