require "uuid"

module Bitte
  class CLI
    register_sub_command provision : Provision, description: "Provision a deployment"

    class Provision < Admiral::Command
      include Helpers

      define_help description: "Initial provisioning from Terraform!"

      def run
        log.info { "Provisioning" }

        create_certs
        create_secrets
        set_ssh_config
        copy_nix
        rebuild
      end

      def create_certs
        generate_ca
        topology.each do |_, node_config|
          # TODO: one for each ASG?
          generate_client_cert(node_config)
          generate_server_cert(node_config)
          generate_pem(node_config)
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

      def copy_nix
        cluster.nodes.each do |name, node|
          log.info { "Copying Nix to #{name} ..." }

          flake_source = IO::Memory.new
          sh! "nix", output: flake_source, args: [
            "eval", "--raw", "#{flake}#self.outPath"
          ]

          sh! "rsync", args: [
            "-e", (["ssh"] + SSH::COMMON_ARGS + ssh_key ).join(" "),
            "-r", "#{ flake_source }/", "root@#{node.public_ip}:source/",
          ]

          nix_flakes = IO::Memory.new
          sh! "nix", output: nix_flakes, args: [
            "eval", "--raw", "#{flake}#nixFlakes.outPath"
          ]

          sh! "nix", "copy",
            "--substitute-on-destination",
            "--to", "ssh://root@#{node.public_ip}",
            "#{flake}#nixFlakes"

          sh! "ssh", args: SSH::COMMON_ARGS + ssh_key + [
            "root@#{node.public_ip}",
            "#{nix_flakes}/bin/nix build --experimental-features 'flakes nix-command' './source#nixosConfigurations.#{cluster_name}-#{name}.config.system.build.toplevel'"
          ]

          sh! "nix", "copy",
            "--substitute-on-destination",
            "--to", "ssh://root@#{node.public_ip}",
            "#{flake}#nixosConfigurations.#{cluster_name}-#{name}.config.system.build.toplevel"

          copy_secrets(node)
        end
      end

      def rebuild
        ch = Channel(Nil).new

        cluster.nodes.each do |name, node|
          spawn do
            begin
            sh! "nixos-rebuild",
              "--flake", "#{flake}##{cluster_name}-#{name}",
              "switch", "--target-host", "root@#{node.public_ip}"
            rescue ex
              log.error(exception: ex) { "nixos-rebuild failed"}
            ensure
              ch.send nil
            end
          end
        end

        cluster.nodes.each do |_, _|
          ch.receive
        end
      end

      # TODO: replace with rsync
      def copy_secrets(node)
        dst = "root@#{node.public_ip}"
        sh! "ssh", dst, "mkdir", "-p", "/etc/consul.d"
        sh! "scp", "./secrets/consul.master.token.json", "#{dst}:/etc/consul.d/master-token.json"
        sh! "scp", "./secrets/#{cluster_name}.pem", "#{dst}:/run/keys/ca.pem"
        sh! "scp", "./encrypted/#{cluster_name}/#{node.name}.enc.json", "#{dst}:/run/keys/certs.enc.json"
        Dir.glob("encrypted/#{cluster_name}/*") do |pem|
          sh! "scp", pem, "#{dst}:/run/keys/#{File.basename(pem)}"
        end
      end

      def cluster
        Cluster.new(
          profile: parent.flags.as(CLI::Flags).profile,
          flake: flake,
          name: cluster_name,
          region: parent.flags.as(CLI::Flags).region
        )
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
            enc.to_s
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

        FileUtils.rm(( secrets/"#{cluster_name}.csr" ).to_s)
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
        File.exists?(secrets / "#{cluster_name}-key.pem") &&
          File.exists?(secrets / "#{cluster_name}.pem")
      end

      def topology
        nix_eval "#{flake}#clusters.#{cluster_name}.topology.nodes" do |output|
          Hash(String, TopologyNode).from_json(output.to_s)
        end
      end

      def cluster_name
        parent.flags.as(CLI::Flags).cluster
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
