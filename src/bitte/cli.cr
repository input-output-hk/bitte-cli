module Bitte
  class CLI < Admiral::Command
    define_version "1.0.0"
    define_help short: "h", description: "Deploy all the things!"

    define_flag cluster : String,
      description: "Name of the cluster",
      default: ENV["BITTE_CLUSTER"]?

    define_flag flake : String,
      description: "Path to the flake",
      default: ENV["BITTE_FLAKE"]? || "."

    # TODO: this should probably be taken from the cluster
    define_flag region : String,
      description: "AWS Region",
      default: ENV["AWS_DEFAULT_REGION"]?

    # TODO: this should probably be taken from the cluster
    define_flag profile : String,
      description: "AWS Profile",
      default: ENV["AWS_PROFILE"]?

    def run
      puts help
    end
  end

  class RetryableError < Exception
  end
end
