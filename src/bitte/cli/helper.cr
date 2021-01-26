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

      def deployer_platform
        {% if flag?(:x86_64) && flag?(:darwin) %}
          "x86_64-darwin"
        {% else %}
          "x86_64-linux"
        {% end %}
      end

      def with_workspace(cluster, workspace_name)
        sh! "nix", "run", ".#nixosConfigurations.#{cluster}-monitoring.config.secrets.preGenerateScript"

        begin
          sh! "nix", "run", ".#clusters.#{deployer_platform}.#{cluster}.tf.#{workspace_name}.config"
        rescue
          # for compatibility
          sh! "nix", "run", ".#clusters.#{cluster}.tf.#{workspace_name}.config"
        end
        original = tf_workspace_show

        if original != workspace_name
          list = tf_workspace_list
          Log.for("terraform").debug{ "found workspaces: #{list.inspect}" }
          tf_workspace_new(cluster, workspace_name) unless list.includes?("#{cluster}_#{workspace_name}")
          tf_workspace_select workspace_name
        else
          sh! "terraform", "init"
        end

        yield
      ensure
        tf_workspace_select original if original && original != workspace_name
      end

      def tf_workspace_select(name) : Nil
        sh! "terraform", "workspace", "select", name
        sh! "terraform", "init"
      end

      def tf_workspace_new(cluster, workspace_name) : Nil
        Log.for("terraform").info { "Creating workspace #{workspace_name} in #{tf_organization}" }
        Bitte::Terraform.create_workspace(tf_organization, "#{cluster}_#{workspace_name}")
      end

      def tf_workspace_show : String
        output = IO::Memory.new
        sh! "terraform", "workspace", "show", output: output
        output.to_s.strip
      end

      def tf_workspace_list : Array(String)
        workspaces = Bitte::Terraform.list_workspaces(tf_organization)
        workspaces ? workspaces.map(&.attributes.name) : Array(String).new
      end

      def tf_organization
        from_env = ENV["TERRAFORM_ORGANIZATION"]?
        return from_env if from_env

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
