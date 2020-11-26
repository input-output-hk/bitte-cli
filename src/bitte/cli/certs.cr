require "json"
require "file_utils"

module Bitte
  class CLI
    register_sub_command certs : Certs, description: "Generate TLS certs"

    class Certs < Admiral::Command
      include Helpers

      define_help short: "h", description: "Generate TLS certs"

      define_flag domain : String, required: true

      def run
        ENV["VAULT_ADDR"] = "https://vault.#{flags.domain}"
        ENV["VAULT_CACERT"] = "secrets/ca.pem"
        ENV["VAULT_FORMAT"] = "json"
        ENV["VAULT_SKIP_VERIFY"] = "true"

        sh! "vault", "login", "-method", "aws", "-no-print"

        mem = IO::Memory.new

        sh! "vault", "write", "pki/intermediate/generate/internal",
          %( common_name="vault.#{flags.domain}" ), output: mem

        csr_container = NamedTuple(data: NamedTuple(csr: String)).from_json(mem.to_s)
        csr = csr_container[:data][:csr]
        File.write("secrets/issuing-ca.csr", csr)

        mem = IO::Memory.new

        sh! "cfssl", "sign",
          "-ca", "secrets/ca.pem",
          "-ca-key", "secrets/ca-key.pem",
          "-hostname", "vault.service.consul",
          "-config", ca_config_file.not_nil!.path,
          "-profile", "intermediate",
          "secrets/issuing-ca.csr", output: mem

        issuing_csr_container = NamedTuple(cert: String).from_json(mem.to_s)
        issuing_pem = issuing_csr_container[:cert]

        File.write("secrets/issuing.pem", issuing_pem.strip)

        File.write("secrets/issuing_full.pem",
          [issuing_pem, File.read("secrets/ca.pem")].map(&.strip).join("\n")
        )

        sh! "vault", "write",
          "pki/intermediate/set-signed",
          "certificate=@secrets/issuing_full.pem"

        full = [
          File.read("secrets/cert.pem"),
          issuing_pem,
          File.read("secrets/ca.pem"),
        ].map(&.strip).join("\n")

        File.write("secrets/full.pem", full)

        sh! "sops", "--set", %(["full"] #{full.to_json}), "encrypted/cert.json"
      end

      def ca_config_file
        File.tempfile "ca-config.json" do |file|
          file.puts(ca_config.to_pretty_json)
        end
      end

      def ca_config
        {
          signing: {
            default: {
              expiry: "87600h",
            },
            profiles: {
              default: {
                usages: ["signing", "key encipherment", "server auth", "client auth"],
                expiry: "8760h",
              },

              intermediate: {
                usages:        ["signing", "key encipherment", "cert sign", "crl sign"],
                expiry:        "43800h",
                ca_constraint: {is_ca: true},
              },
            },
          },
        }
      end
    end
  end
end
