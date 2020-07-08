module Bitte
  class CLI
    module Helpers
      def sh!(cmd : String,
              args : Enumerable(String)?,
              env : Process::Env = nil,
              input : Process::Stdio = Process::Redirect::Pipe,
              output : IO = LogIO.new(log),
              error : IO = LogIO.new(log))

        log.debug { "run: #{cmd} #{args.join(" ")}" }

        Process.run(
          cmd,
          args: args,
          env: env,
          input: input,
          output: output,
          error: error,
        ) do |process|
          yield process
        end

        raise RetryableError.new("Process exited with #{$?.exit_status}") unless $?.success?
      end

      def sh!(cmd : String,
              args : Enumerable(String)?,
              env : Process::Env = nil,
              input : Process::Stdio = Process::Redirect::Close,
              output : IO = LogIO.new(log),
              error : IO = LogIO.new(log))
        sh!(cmd, args: args, env: env, input: input, output: output, error: error){|_| }
      end

      def sh!(cmd : String,
              *args : String,
              env : Process::Env = nil,
              input : Process::Stdio = Process::Redirect::Close,
              output : IO = LogIO.new(log),
              error : IO = LogIO.new(log))
        sh!(cmd, args: args.to_a, env: env, input: input, output: output, error: error){|_| }
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
        path = secrets/"ssh-#{cluster.name}"
        if File.exists?(path.to_s)
          ["-i", path.to_s]
        else
          [] of String
        end
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
        Log.info { "Connecting to #{ip}:22..." }

        120.downto(0) do |i|
          begin
            TCPSocket.open(ip, 22) do
              log.info { "Connected to #{ip}." }
            end

            return
          rescue Socket::ConnectError
            log.debug { "Connection to #{ip} failed again. #{i} attempts remaining" }
            sleep 1
          end
        end

        raise "Couldn't connect to #{ip}"
      end
    end
  end
end
