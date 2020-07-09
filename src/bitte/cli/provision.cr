require "uuid"

module Bitte
  class CLI
    register_sub_command provision : Provision, description: "Provision a deployment"

    class Provision < Admiral::Command
      include Helpers

      define_help description: "Initial provisioning from Terraform!"

      property cluster : TerraformCluster?

      # TODO: fix race
      property log_name : String?

      def run
        log.info { "Provisioning" }

        set_ssh_config

        # TODO: add the encrypt key here
        create_secrets
        generate_ca

        cluster.instances.each do |name, instance|
          @log_name = name
          wait_for_ssh(instance.public_ip)
          generate_client_cert(instance)
          generate_server_cert(instance)
          generate_pem(instance)
          copy_secrets(instance)

          # # TODO: make this parallel once https://github.com/NixOS/nix/issues/3794 is fixed
          # copy_nix(instance)
          #
          # spawn do
          #   begin
          #     build(instance)
          #     switch(instance)
          #   rescue ex
          #     log.error(exception: ex) { "error during provision" }
          #   ensure
          #     ch.send nil
          #   end
          # end
        end
      end

      def create_secrets
        unless File.exists?("./secrets/consul.master.token.json")
          json = {acl: {tokens: {master: UUID.random.to_s}}}.to_pretty_json
          File.write("./secrets/consul.master.token.json", json)
        end
      end

      def set_ssh_config
        ENV["NIX_SSHOPTS"] ||= (SSH::COMMON_ARGS + ssh_key).join(" ")
      end

      # TODO: replace with rsync
      def copy_secrets(instance)
        dst = "root@#{instance.public_ip}"
        sh! "ssh", args: SSH::COMMON_ARGS + ssh_key + [ dst, "mkdir", "-p", "/etc/consul.d" ]
        sh! "scp", (secrets/"consul.master.token.json" ).to_s, "#{dst}:/etc/consul.d/master-token.json"
        sh! "scp", (secrets/"#{cluster.name}.pem").to_s, "#{dst}:/run/keys/ca.pem"
        sh! "scp", (encrypted/cluster.kms/"#{instance.name}.enc.json").to_s, "#{dst}:/run/keys/certs.enc.json"
        sh! "scp", (encrypted/cluster.kms/"client.enc.json").to_s, "#{dst}:/run/keys/client.enc.json"
        Dir.glob(( encrypted/cluster.name/"*" )) do |file|
          sh! "scp", file, "#{dst}:/run/keys/#{File.basename(file)}"
        end
        Dir.glob(( encrypted/cluster.kms/"*" )) do |file|
          sh! "scp", file, "#{dst}:/run/keys/#{File.basename(file)}"
        end
      end

      def generate_client_cert(instance)
        enc = encrypted/cluster.kms/"client.enc.json"

        if File.exists?(enc) && mtime(secrets/"#{cluster.name}-key.pem") <= mtime(enc)
          return
        end

        config = ca_config_file
        cert_config = cert_config_file_client(instance)

        FileUtils.mkdir_p(enc.parent.to_s)

        File.open(enc, "w+") do |sopsfile|
          sh!("sops", output: sopsfile, args: [
            "--encrypt",
            "--input-type", "json",
            "--kms", cluster.kms,
            "/dev/stdin",
          ]) do |sops|
            # we could plug in a multiwriter here that extracts the pem so we
            # don't have to call sops twice...
            sh!("cfssl", output: sops.input, args: [
              "gencert",
              "-ca", (secrets / "#{cluster.name}.pem").to_s,
              "-ca-key", (secrets / "#{cluster.name}-key.pem").to_s,
              "-config", config.not_nil!.path,
              "-profile", "default",
              cert_config.not_nil!.path,
            ])
          end
        end
      ensure
        [config, cert_config].compact.each(&.delete)
      end


      def generate_server_cert(instance)
        enc = encrypted/cluster.kms/"#{instance.name}.enc.json"

        if File.exists?(enc) && mtime(secrets/"#{cluster.name}-key.pem") <= mtime(enc)
          return
        end

        config = ca_config_file
        cert_config = cert_config_file_server(instance)

        FileUtils.mkdir_p(enc.parent.to_s)

        File.open(enc, "w+") do |sopsfile|
          sh!("sops", output: sopsfile, args: [
            "--encrypt",
            "--input-type", "json",
            "--kms", cluster.kms,
            "/dev/stdin",
          ]) do |sops|
            # we could plug in a multiwriter here that extracts the pem so we
            # don't have to call sops twice...
            sh!("cfssl", output: sops.input, args: [
              "gencert",
              "-ca", (secrets / "#{cluster.name}.pem").to_s,
              "-ca-key", (secrets / "#{cluster.name}-key.pem").to_s,
              "-config", config.not_nil!.path,
              "-profile", "default",
              cert_config.not_nil!.path,
            ])
          end
        end
      ensure
        [config, cert_config].compact.each(&.delete)
      end

      def generate_pem(instance)
        pem = encrypted/cluster.name/"#{instance.name}.pem"
        enc = encrypted/cluster.kms/"#{instance.name}.enc.json"

        if File.exists?(pem) && mtime(enc) <= mtime(pem)
          return
        end

        FileUtils.mkdir_p(pem.parent.to_s)
        FileUtils.mkdir_p(enc.parent.to_s)

        File.open(pem, "w+") do |pem_file|
          sh!("sops", output: pem_file, args: [
            "--decrypt",
            "--extract", %(["cert"]),
            enc.to_s
          ])
        end
      end

      def cert_config_file_server(instance)
        File.tempfile "#{instance.name}.json" do |file|
          file.puts({
            CN:    "#{instance.name}.node.consul",
            names: ca_names,
            key:   ca_key,
            hosts: [
              instance.name,
              "#{instance.name}.node.consul",
              "vault.service.consul",
              "consul.service.consul",
              "nomad.service.consul",
              "server.#{cluster.region}.consul",
              "127.0.0.1",
              instance.private_ip,
            ],
          }.to_pretty_json)
        end
      end

      def cert_config_file_client(instance)
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

        sh!("cfssljson", args: ["-bare", (secrets/cluster.name).to_s]) do |cfssljson|
          sh!("cfssl", ["gencert", "-initca", ca_tempfile.path], output: cfssljson.input)
        end

        FileUtils.rm(( secrets/"#{cluster.name}.csr" ).to_s)
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
        File.exists?(secrets / "#{cluster.name}-key.pem") &&
          File.exists?(secrets / "#{cluster.name}.pem")
      end

      def log_name
        @log_name ||= self.class.to_s
      end

      def cluster
        @cluster ||= TerraformCluster.load
      end

      def cluster_name
        parent.flags.as(CLI::Flags).cluster
      end

      def flake
        parent.flags.as(CLI::Flags).flake
      end
    end
  end
end
