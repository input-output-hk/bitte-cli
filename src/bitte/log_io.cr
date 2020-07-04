require "log"

class LogIO < IO
  include IO::Buffered

  getter closed = false

  def initialize(@log : Log)
    super()
    self.flush_on_newline = true
  end

  def unbuffered_write(data)
    String.new(data).each_line do |line|
      @log.info { line }
    end
    0i64
  end

  def unbuffered_flush
  end

  def unbuffered_close
    pp! :close
  end

  def unbuffered_rewind
    pp! :rewind
  end

  def unbuffered_read(data)
    raise "can't read from LogIO"
  end
end
