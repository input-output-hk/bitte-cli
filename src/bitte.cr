require "./cli.cr"

Log.setup_from_env(
  default_level: Log::Severity::Info,
  backend: Log::IOBackend.new(STDERR)
)

Bitte::CLI.run
