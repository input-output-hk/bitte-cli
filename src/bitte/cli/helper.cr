module Bitte
  class CLI
    module Helpers
      def sh!(cmd : String, args : Array( String ), output : IO, &block : Process -> _)
        log.debug { "run: #{cmd} #{args.to_a.join(" ")}" }
        Process.run(cmd, args: args.to_a, output: output, error: LogIO.new(log), &block)
        raise RetryableError.new("Process exited with #{$?.exit_status}") unless $?.success?
      end

      def sh!(cmd : String, args : Array( String ), output : IO)
        log.debug { "run: #{cmd} #{args.to_a.join(" ")}" }
        Process.run(cmd, args: args.to_a, output: output, error: LogIO.new(log))
        raise RetryableError.new("Process exited with #{$?.exit_status}") unless $?.success?
      end

      def sh!(cmd : String, *args : String, &block)
        log.debug { "run: #{cmd} #{args.to_a.join(" ")}" }
        output = IO::Memory.new
        Process.run(cmd, args.to_a, output: output, error: LogIO.new(log))
        raise RetryableError.new("Process exited with #{$?.exit_status}") unless $?.success?
        yield output
      end

      def sh!(cmd : String, *args : String)
        log.debug { "run: #{cmd} #{args.to_a.join(" ")}" }
        Process.run(cmd, args.to_a, output: LogIO.new(log), error: LogIO.new(log))
        raise RetryableError.new("Process exited with #{$?.exit_status}") unless $?.success?
      end

      def ssh_key
        path = "./secrets/ssh-#{cluster}"
        if File.exists?(path)
          ["-i", path]
        else
          [] of String
        end
      end

      def log
        Log.for(self.class.to_s)
      end

      def nix_eval(attr, apply)
        sh! "nix", "eval", "--json", attr, "--apply", apply do |output|
          yield output
        end
      end

      def nix_eval(attr)
        sh! "nix", "eval", "--json", attr do |output|
          yield output
        end
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
