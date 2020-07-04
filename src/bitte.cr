require "admiral"

module Bitte
end

if ARGV == ["schön"]
  puts "Danke sehr"
  exit
end

require "./bitte/cli.cr"
require "./bitte/cli/helper.cr"
require "./bitte/**"

Log.setup_from_env(default_level: Log::Severity::Info)

Bitte::CLI.run
