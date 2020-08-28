module Bitte
  class CLI
    register_sub_command job : Job, description: "Run Nomad Jobs"

    class Job < Admiral::Command
      include Helpers

      define_help short: "h", description: "Run Nomad Jobs"

      define_argument job : String, required: true
      define_flag flake : String, default: "."

      def run
        url = URI.parse("s3://#{s3_bucket}")
        url.query = HTTP::Params{
          "profile" => aws_profile,
          "region" => aws_region,
        }.to_s

        sh! "nix", "copy", "--to", url.to_s, "#{flake}##{flake_attr}"
        sh! "nix", "build", "#{flake}##{flake_attr}", "-o", "job.nomad.json"

        # sh! "nomad", "job", "run", job_file
      end

      def flake : String
        flags.flake || "."
      end

      def flake_attr
        "clusters.#{cluster_name}.jobs.#{arguments.job}"
      end

      def cluster_name
        ENV["BITTE_CLUSTER"]
      end

      def s3_bucket : String
        ENV["BITTE_CACHE_S3_BUCKET"]? || raise "BITTE_CACHE_S3_BUCKET must be set"
      end

      def aws_profile : String
        ENV["AWS_PROFILE"]? || raise "AWS_PROFILE must be set"
      end

      def aws_region : String
        aws_region_env || aws_region_config
      end

      def aws_region_env : String?
        ENV["AWS_DEFAULT_REGION"]?
      end

      def aws_region_config : String
        output = IO::Memory.new
        sh! "aws", "configure", "get", "region", "--profile", aws_profile, output: output
        output.to_s.strip
      end
    end
  end
end
