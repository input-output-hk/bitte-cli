require "admiral"
require "json"

module Bitte
  class TopologyNode
    include JSON::Serializable

    # property kms : String
    property region : String
    property name : String

    @[JSON::Field(key: "privateIP")]
    property private_ip : String
  end
end

if ARGV == ["schön"]
  puts "Danke sehr"
  exit
end

if ARGV == ["sehr"]
  puts "Danke schön"
  exit
end

if ARGV.empty?
  puts "Danke"
  exit
end

require "./bitte/cli.cr"
require "./bitte/cli/helper.cr"
require "./bitte/**"

Log.setup_from_env(
  default_level: Log::Severity::Info,
  backend: Log::IOBackend.new(STDERR)
)

Bitte::CLI.run
