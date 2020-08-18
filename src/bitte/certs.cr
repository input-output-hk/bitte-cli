require "json"
require "file_utils"

module Bitte
  class CLI
    register_sub_command certs : Certs, description: "Generate TLS certs"

    class Certs < Admiral::Command
      include Helpers

      define_help short: "h", description: "Generate TLS certs"

      define_flag domain : String, required: true

      CSR_CONTAINER = NamedTuple(data: NamedTuple(csr: String))

      def run
        ENV["VAULT_ADDR"] = "https://3.121.27.212:8200"
        ENV["VAULT_CACERT"] = "secrets/ca.pem"
        ENV["VAULT_FORMAT"] = "json"
        sh! "vault", "login", "-method", "aws", "-no-print"

        mem = IO::Memory.new

        sh! "vault", "write", "pki/intermediate/generate/internal",
          %( common_name="vault.#{flags.domain}" ), output: mem

        csr_container = CSR_CONTAINER.from_json(mem.to_s)
        csr = csr_container[:data][:csr]
        File.write("secrets/issuing-ca.csr", csr)

        sh! "cfssl", "sign",
          "-ca", "secrets/ca.pem",
          "-ca-key", "secrets/ca-key.pem",
          "-hostname", "vault.service.consul",
          "-config", ca_config_file.not_nil!.path,
          "-profile", "intermediate",
          "secrets/issuing-ca.csr", output: mem

        issuing_csr_container = CSR_CONTAINER.from_json(mem.to_s)
        issuing_csr = issuing_csr_container[:data][:csr]

        issuing = issuing_csr.gsub(/\n/, "")
        issuing += File.read("secrets/ca.pem")
        File.write("secrets/issuing.pem", issuing)

        sh! "vault", "write", "pki/intermediate/set-signed", "certificate=@secrets/issuing.pem"
      end

      def generate_client_cert(node)
        enc = encrypted/cluster_name/"client.enc.json"

        if File.exists?(enc) && mtime(secrets/"#{cluster_name}-key.pem") <= mtime(enc)
          return
        end

        config = ca_config_file
        cert_config = cert_config_file_client(node)

        FileUtils.mkdir_p((encrypted/cluster_name).to_s)

        File.open(enc, "w+") do |sopsfile|
          sh!("sops", output: sopsfile, args: [
            "--encrypt",
            "--input-type", "json",
            "--kms", node.kms,
            "/dev/stdin",
          ]) do |sops|
            # we could plug in a multiwriter here that extracts the pem so we
            # don't have to call sops twice...
            sh!("cfssl", output: sops.input, args: [
              "gencert",
              "-ca", (secrets / "#{cluster_name}.pem").to_s,
              "-ca-key", (secrets / "#{cluster_name}-key.pem").to_s,
              "-config", config.not_nil!.path,
              "-profile", "default",
              cert_config.not_nil!.path,
            ])
          end
        end
      ensure
        [config, cert_config].compact.each(&.delete)
      end

      def generate_server_cert(node)
        enc = encrypted/cluster_name/"#{node.name}.enc.json"

        if File.exists?(enc) && mtime(secrets/"#{cluster_name}-key.pem") <= mtime(enc)
          return
        end

        config = ca_config_file
        cert_config = cert_config_file_server(node)

        FileUtils.mkdir_p((encrypted/cluster_name).to_s)

        File.open(enc, "w+") do |sopsfile|
          sh!("sops", output: sopsfile, args: [
            "--encrypt",
            "--input-type", "json",
            "--kms", node.kms,
            "/dev/stdin",
          ]) do |sops|
            # we could plug in a multiwriter here that extracts the pem so we
            # don't have to call sops twice...
            sh!("cfssl", output: sops.input, args: [
              "gencert",
              "-ca", (secrets / "#{cluster_name}.pem").to_s,
              "-ca-key", (secrets / "#{cluster_name}-key.pem").to_s,
              "-config", config.not_nil!.path,
              "-profile", "default",
              cert_config.not_nil!.path,
            ])
          end
        end
      ensure
        [config, cert_config].compact.each(&.delete)
      end

      def generate_pem(node)
        pem = encrypted/cluster_name/"#{node.name}.pem"
        enc = encrypted/cluster_name/"#{node.name}.enc.json"

        if File.exists?(pem) && mtime(enc) <= mtime(pem)
          return
        end

        File.open(pem, "w+") do |pem_file|
          sh!("sops", output: pem_file, args: [
            "--decrypt",
            "--extract", %(["cert"]),
            enc.to_s,
          ])
        end
      end

      def cert_config_file_server(node)
        File.tempfile "#{node.name}.json" do |file|
          file.puts({
            CN:    "#{node.name}.node.consul",
            names: ca_names,
            key:   ca_key,
            hosts: [
              node.name,
              "#{node.name}.node.consul",
              "vault.service.consul",
              "consul.service.consul",
              "nomad.service.consul",
              "server.#{node.region}.consul",
              "127.0.0.1",
              node.private_ip,
            ],
          }.to_pretty_json)
        end
      end

      def cert_config_file_client(node)
        File.tempfile "client.json" do |file|
          file.puts({
            CN:    "client.node.consul",
            names: ca_names,
            key:   ca_key,
            hosts: [
              "127.0.0.1",
            ],
          }.to_pretty_json)
        end
      end

      def generate_ca
        return if ca_exists?

        ca_tempfile = File.tempfile "ca.json" do |file|
          file.puts({
            hosts: ["consul"],
            names: ca_names,
            key:   ca_key,
          }.to_pretty_json)
        end

        FileUtils.mkdir_p(secrets.to_s)

        sh!("cfssljson", args: ["-bare", (secrets/cluster_name).to_s]) do |cfssljson|
          sh!("cfssl", ["gencert", "-initca", ca_tempfile.path], output: cfssljson.input)
        end

        FileUtils.rm((secrets/"#{cluster_name}.csr").to_s)
      ensure
        ca_tempfile.delete if ca_tempfile
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

      def ca_key
        {algo: "rsa", size: 2048}
      end

      def ca_names
        [{
          O:  "IOHK",
          C:  "JP",
          ST: "Kantō",
          L:  "Tokyo",
        }]
      end

      def ca_exists?
        File.exists?(secrets / "#{cluster_name}-key.pem") &&
          File.exists?(secrets / "#{cluster_name}.pem")
      end

      def topology
        nix_eval "#{flake}#clusters.#{cluster_name}.topology.nodes" do |output|
          Hash(String, TopologyNode).from_json(output.to_s)
        end
      end

      def flake
        parent.flags.as(CLI::Flags).flake
      end

      def cluster_name
        parent.flags.as(CLI::Flags).cluster
      end
    end
  end
end
