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
              logger : Log? = self.log,
             )

        logger.debug { "run: #{cmd} #{args.join(" ")}" }

        Process.run(
          cmd,
          args: args,
          env: env,
          input: input,
          output: output || LogIO.new(logger),
          error: error || LogIO.new(logger),
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
        sh!(cmd, args: args, env: env, input: input, output: output, error: error, logger: logger){|_| }
      end

      def sh!(cmd : String,
              *args : String,
              env : Process::Env = nil,
              input : Process::Stdio = Process::Redirect::Close,
              output : IO? = nil,
              error : IO? = nil,
              logger : Log? = self.log)
        sh!(cmd, args: args.to_a, env: env, input: input, output: output, error: error, logger: logger){|_| }
      end

      def sh!(cmd : String,
              *args : String,
              env : Process::Env = nil,
              input : Process::Stdio = Process::Redirect::Close,
              output : IO = LogIO.new(log),
              error : IO = LogIO.new(log))
        sh!(cmd, args: args.to_a, env: env, input: input, output: output, error: error){|process| yield process }
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

        if original == name
          return yield
        end

        available = tf_workspace_list

        if available.includes?(name)
          tf_workspace_select name
        else
          tf_workspace_new name
        end

        yield
      ensure
        tf_workspace_select original if original
      end

      def tf_workspace_select(name) : Nil
        sh! "terraform", "workspace", "select", name
      end

      def tf_workspace_new(name) : Nil
        sh! "terraform", "workspace", "new", name
      end

      def tf_workspace_show : String
        output = IO::Memory.new
        sh! "terraform", "workspace", "show", output: output
        output.to_s.strip
      end

      def tf_workspace_list : Array(String)
        output = IO::Memory.new
        sh! "terraform", "workspace", "list", output: output
        output.to_s.split - ["*"]
      end
    end
  end
end
