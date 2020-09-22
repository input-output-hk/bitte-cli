require "json"

module Bitte
  module Terraform
    def self.log
      Log.for("terraform")
    end

    def self.token
      creds = Hash(String, Hash(String, Hash(String, String))).from_json(
        File.read(File.expand_path("~/.terraform.d/credentials.tfrc.json", home: true))
      )

      creds["credentials"]["app.terraform.io"]["token"]
    rescue ex
      log.error(exception: ex) {
        "Could not found Terraform credentials, make sure you ran `terraform login`"
      }
    end

    def self.headers
      HTTP::Headers{
        "Authorization" => "Bearer #{token}",
        "Content-Type"  => "application/vnd.api+json",
      }
    end

    def self.show_workspace(org, name)
      url = "https://app.terraform.io/api/v2/organizations/#{org}/workspaces/#{name}"
      log.debug { "GET #{url}" }
      HTTP::Client.get( url, headers: headers )
    end

    def self.current_state_version(org, name)
      res = show_workspace(org, name)

      related = JSON.parse(res.body.to_s)["data"]["relationships"]["current-state-version"]["links"]["related"].as_s

      HTTP::Client.get(
        "https://app.terraform.io#{related}",
        headers: headers
      )
    end

    def self.current_state_version_output(org, cluster_name, name)
      res = current_state_version(org, "#{cluster_name}_#{name}")
      state_id = JSON.parse(res.body.to_s)["data"]["relationships"]["outputs"]["data"][0]["id"].as_s

      res = HTTP::Client.get(
        "https://app.terraform.io/api/v2/state-version-outputs/#{state_id}",
        headers: headers
      )

      res.body.to_s
    end

    def self.workspaces(org)
      res = HTTP::Client.get(
        "https://app.terraform.io/api/v2/organizations/#{org}/workspaces",
        headers: headers
      )

      Bitte::Terraform::Workspaces.from_json(res.body.to_s)
    rescue ex
      Log.for("terraform").error(exception: ex) {
        "Could not parse JSON"
      }
      nil
    end

    def self.create_workspace(org, name)
      HTTP::Client.post(
        "https://app.terraform.io/api/v2/organizations/#{org}/workspaces",
        headers: headers,
        body: {
          data: {
            attributes: {
              name:       name,
              operations: false,
            },
            type: "workspaces",
          },
        }.to_json
      )
    end

    def self.list_workspaces(org)
      workspaces(org).try(&.data)
    end

    def self.localize_workspaces
      workspaces.data.each do |workspace|
        next unless required.includes?(workspace.attributes.name)

        required.delete workspace.attributes.name

        next if workspace.attributes.execution_mode == "local"

        HTTP::Client.patch(
          "https://app.terraform.io/api/v2/organizations/#{tf_organization}/workspaces/#{workspace.id}",
          headers: headers,
          body: {
            data: {
              type:       "workspace",
              attributes: {
                operations: false,
              },
            },
          }.to_json
        )
      end
    end

    class Workspaces
      include JSON::Serializable

      property data : Array(Workspace)

      class Workspace
        include JSON::Serializable

        property id : String
        property attributes : Attributes
        property relationships : Relationships
        property links : WorkspaceLinks

        @[JSON::Field(key: "type")]
        property workspace_type : String
      end

      class Attributes
        include JSON::Serializable

        property actions : Actions
        property description : Nil
        property environment : String
        property locked : Bool
        property name : String
        property operations : Bool
        property permissions : Hash(String, Bool)
        property source : String

        @[JSON::Field(key: "auto-apply")]
        property auto_apply : Bool

        @[JSON::Field(key: "created-at")]
        property created_at : String

        @[JSON::Field(key: "queue-all-runs")]
        property queue_all_runs : Bool

        @[JSON::Field(key: "terraform-version")]
        property terraform_version : String

        @[JSON::Field(key: "working-directory")]
        property working_directory : String?

        @[JSON::Field(key: "speculative-enabled")]
        property speculative_enabled : Bool

        @[JSON::Field(key: "allow-destroy-plan")]
        property allow_destroy_plan : Bool

        @[JSON::Field(key: "auto-destroy-at")]
        property auto_destroy_at : Nil

        @[JSON::Field(key: "latest-change-at")]
        property latest_change_at : String

        @[JSON::Field(key: "execution-mode")]
        property execution_mode : String

        @[JSON::Field(key: "vcs-repo")]
        property vcs_repo : Nil

        @[JSON::Field(key: "vcs-repo-identifier")]
        property vcs_repo_identifier : Nil

        @[JSON::Field(key: "file-triggers-enabled")]
        property file_triggers_enabled : Bool

        @[JSON::Field(key: "trigger-prefixes")]
        property trigger_prefixes : Array(JSON::Any?)

        @[JSON::Field(key: "source-name")]
        property source_name : Nil

        @[JSON::Field(key: "source-url")]
        property source_url : Nil
      end

      class Actions
        include JSON::Serializable

        @[JSON::Field(key: "is-destroyable")]
        property is_destroyable : Bool
      end

      class WorkspaceLinks
        include JSON::Serializable

        @[JSON::Field(key: "self")]
        property links_self : String
      end

      class Relationships
        include JSON::Serializable

        property organization : AgentPool

        @[JSON::Field(key: "locked-by")]
        property locked_by : CurrentStateVersion?

        @[JSON::Field(key: "current-run")]
        property current_run : AgentPool

        @[JSON::Field(key: "latest-run")]
        property latest_run : AgentPool

        @[JSON::Field(key: "agent-pool")]
        property agent_pool : AgentPool
      end

      class AgentPool
        include JSON::Serializable

        property data : Data?
      end

      class Data
        include JSON::Serializable

        property id : String

        @[JSON::Field(key: "type")]
        property data_type : String
      end

      class CurrentStateVersion
        include JSON::Serializable

        property data : Data
        property links : CurrentStateVersionLinks
      end

      class CurrentStateVersionLinks
        include JSON::Serializable

        property related : String
      end
    end
  end
end
