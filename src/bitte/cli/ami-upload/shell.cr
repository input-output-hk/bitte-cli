module Bitte
  module AMI
    module Shell
      def sh_silent!(cmd, *args)
        output = IO::Memory.new
        Process.run(cmd, args, output: output, error: STDERR).tap do |status|
          raise "#{cmd} #{args} failed" unless status.success?
        end
        output.to_s.strip
      end

      def sh!(cmd, *args)
        puts "$ #{cmd} #{args.to_a.join(" ")}"
        output = IO::Memory.new
        Process.run(cmd, args, output: output, error: STDERR).tap do |status|
          raise "#{cmd} #{args} failed" unless status.success?
        end
        output.to_s.strip
      end

      def sh(cmd, *args)
        puts "$ #{cmd} #{args.to_a.join(" ")}"
        output = IO::Memory.new
        status = Process.run(cmd, args, output: output, error: STDERR)
        output.to_s.strip if status.success?
      end

      def sh_silent(cmd, *args)
        output = IO::Memory.new
        status = Process.run(cmd, args, output: output, error: STDERR)
        output.to_s.strip if status.success?
      end
    end
  end
end
