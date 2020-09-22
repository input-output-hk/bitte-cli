module Bitte
  module AMI
    module Common
      def perf_uuid
        file = "perf-uuid.txt"

        if ENV["PERF_UUID"]?
          ENV["PERF_UUID"]
        elsif File.exists?(file) && !File.empty?(file)
          File.read(file).strip
        else
          "undefined"
        end
      end

      def validate_regions(regions)
        invalid = regions - ALL_REGIONS
        raise "invalid regions: #{invalid.join(", ")}" unless invalid.empty?
      end

      def bucket_region : String
        parent.flags.as(CLI::AMI::Flags).bucket_region
      end

      def bucket : String
        parent.flags.as(CLI::AMI::Flags).bucket
      end
    end
  end
end
