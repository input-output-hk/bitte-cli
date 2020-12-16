module Bitte
  class CLI
    register_sub_command job : Job, description: "Run Nomad Jobs"

    class Job < Admiral::Command
      include Helpers

      define_help short: "h", description: "Run Nomad Jobs"

      define_argument job : String, required: true

      def run
        sh! "nix", "run", ".#nomadJobs.#{arguments.job}.run"
      end
    end
  end
end
