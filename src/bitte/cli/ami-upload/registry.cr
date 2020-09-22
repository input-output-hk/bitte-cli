require "json"
require "./common"
require "./shell"

module Bitte
  module AMI
    class Registry
      class Images
        include JSON::Serializable

        property images : Hash(String, State)

        def []?(key)
          @images[key]?
        end

        def [](key)
          @images[key] ||= State.new(nil, nil, nil)
        end

        def []=(key, value)
          @images[key] = value
        end
      end

      class State
        include JSON::Serializable

        property task_id : String?
        property snapshot_id : String?
        property ami_id : String?

        def initialize(@task_id, @snapshot_id, @ami_id)
        end
      end

      getter path : String

      def initialize(@path)
        File.write(@path, %({"images":{}})) unless File.file?(@path)
      end

      def open
        images = Images.from_json(File.read(@path))
        yield images
      ensure
        File.write(@path, images.to_pretty_json)
      end
    end

    class ImageInfo
      include ::Bitte::AMI::Common
      include ::Bitte::AMI::Shell
      include JSON::Serializable

      property label : String
      property system : String
      property logical_bytes : String
      property file : String
      property nix_path : String? = nil

      # Round to the next GB
      def logical_gigabytes
        ((logical_bytes.to_u64 - 1) / 1024 / 1024 / 1024 + 1).ceil.to_u64
      end

      def amazon_arch
        case system
        when "aarch64-linux"
          "arm64"
        when "x86_64-linux"
          "x86_64"
        else
          raise "unknown system '#{system}'"
        end
      end

      def name
        file.split("/")[3]
      end

      def description
        "NixOS #{label} #{system}"
      end

      def state_key(region)
        "#{name}-#{region}"
      end

      def self.from_nix_path(path : String)
        from_json(File.read(File.join(path, "/nix-support/image-info.json")))
      end

      def self.prepare(ami)
        pp! ami
        if path = sh("nix-build", "--no-out-link", "./.", "-A", "amis.#{ami}")
          ImageInfo.from_nix_path(path)
        else
          raise "Couldn't build image"
        end
      end

      def upload_all!(home_region, regions, force)
        home_image_id = upload_image(home_region, force)
        (regions - [home_region]).each do |region|
          copy_to_region region, home_region, home_image_id.not_nil!, force
        end
      end

      def s3_name
        file.lstrip("/")
      end

      def s3_url
        "s3://#{bucket}/#{s3_name}"
      end

      def with_image(region)
        Registry.new("state.json").open do |images|
          yield images[state_key(region)]
        end
      end

      def upload_image(region, force)
        upload_image_import(region, force)
        upload_image_snapshot(region, force)
        upload_image_deregister(region, force)
        upload_image_register(region, force)
      end

      def upload_image_snapshot(region, force)
        with_image region do |image|
          return if !force && image.snapshot_id

          puts "Waiting for import"

          image.snapshot_id = wait_for_import(region, image.task_id.not_nil!)
          if sh!("aws", "ec2", "create-tags",
               "--region", region,
               "--resources", "#{image.snapshot_id}",
               "--tags", "Key=Name,Value=\"#{name}\"",
               "Key=perf-ops,Value=\"#{perf_uuid}\"")
            puts "Snapshot tag successfully created"
          else
            puts "Snapshot tagging failed."
          end
          puts
        end
      end

      def upload_image_deregister(region, force)
        deregister_by_name(region, force)
      end

      def validate_ami_by_id(ami_id, region)
        images = AWS::Client.new(region).describe_images("image-id": ami_id)
        images.images.size == 1
      end

      def upload_image_register(region, force)
        with_image region do |image|
          if !force && image.ami_id
            return image.ami_id if validate_ami_by_id(image.ami_id, region)
          end
          if force
            puts "Forcing a new ami registration"
          end
          ebs = {
            "SnapshotId"          => image.snapshot_id,
            "VolumeSize"          => logical_gigabytes,
            "DeleteOnTermination" => true,
            "VolumeType"          => "gp2",
          }

          block_device_mappings = [
            {"DeviceName" => "/dev/xvda", "Ebs" => ebs},
            {"DeviceName" => "/dev/sdb", "VirtualName" => "ephemeral0"},
            {"DeviceName" => "/dev/sdc", "VirtualName" => "ephemeral1"},
            {"DeviceName" => "/dev/sdd", "VirtualName" => "ephemeral2"},
            {"DeviceName" => "/dev/sde", "VirtualName" => "ephemeral3"},
          ].to_json

          ami_id_output = sh! "aws", "ec2", "register-image",
            "--name", name,
            "--description", description,
            "--region", region,
            "--architecture", amazon_arch,
            "--block-device-mappings", block_device_mappings,
            "--root-device-name", "/dev/xvda",
            "--sriov-net-support", "simple",
            "--ena-support",
            "--virtualization-type", "hvm"
          result = RegisterImageResult.from_json(ami_id_output)
          image.ami_id = result.image_id
          puts "#{region} AMI ID: #{image.ami_id}"
          puts

          if sh!("aws", "ec2", "create-tags",
               "--region", region,
               "--resources", "#{image.ami_id}",
               "--tags", "Key=Name,Value=\"#{name}\"",
               "Key=perf-ops,Value=\"#{perf_uuid}\"",
               "Key=source-region,Value=\"#{region}\"",
               "Key=snapshot-id,Value=\"#{image.snapshot_id}\"",
               "Key=ami,Value=\"#{image.ami_id}\"")
            puts "Snapshot tag successfully created"
          else
            puts "Snapshot tagging failed."
          end
          puts
          return image.ami_id
        end
      end

      def deregister_by_name(region, force)
        output = sh "aws", "ec2", "describe-images",
          "--region", region,
          "--filters", "Name=name,Values=\"#{name}\""
        puts
        images = Hash(String, Array(ImageDescription)).from_json(output.to_s)

        if force
          images["Images"].each do |image|
            puts "Deregister #{image.image_id} in #{region}"
            sh! "aws", "ec2", "deregister-image",
              "--region", region,
              "--image-id", image.image_id
            puts
          end
        else
          images["Images"].each do |image|
            with_image region do |image_registry|
              if image_registry.ami_id != image.image_id
                puts "Updating the registry file with the hash matched image #{image.image_id} in #{region}"
                image_registry.ami_id = image.image_id
              else
                puts "Using an existing AMI for #{region} in the registry file: #{image.image_id}"
              end
            end
          end
        end
      end

      def wait_for_import(region, task_id)
        puts "Waiting for import task #{task_id} to be completed"

        loop do
          output = shSilent! "aws", "ec2", "describe-import-snapshot-tasks",
            "--region", region,
            "--import-task-ids", task_id

          tasks = ImportSnapshotTasks.from_json(output)
          task = tasks.import_snapshot_tasks.first.snapshot_task_detail

          case task.status
          when "active"
            print ("%4s/100 : %s" % [task.progress, task.status_message]).ljust(60) + "\r"
            sleep 10
          when "completed"
            puts
            puts
            return task.snapshot_id
          else
            puts
            raise "Unexpected snapshot import status for #{task_id}: #{task.inspect}"
          end
        end
      end

      def upload_image_import(region, force)
        with_image region do |image|
          check_for_s3image(region, force)
          puts
          if !force
            puts "Checking for an existing snapshot in the home region"
            existing_snapshot = sh! "aws", "ec2", "describe-snapshots",
              "--region", region, "--filters", "Name=tag:Name,Values=\"#{name}\""
            puts

            # Check for an existing snapshot with the image hash; use it if it exists
            snapshot_match = SnapshotDescribe.from_json(existing_snapshot).matches
            if snapshot_match.size >= 1 && snapshot_match.first.state != "failed"
              if snapshot_match.size == 1
                puts "Found an existing snapshot with the expected hash in the home region.  " \
                     "Using the existing snapshot: #{snapshot_match.first.snapshotId}"
              else
                puts "Warning: multiple snapshots with the same hash found in the home region.  " \
                     "Using the first snapshot found: #{snapshot_match.first.snapshotId}"
              end
              image.snapshot_id = snapshot_match.first.snapshotId
              puts
              return
            end
          else
            puts "Force pushing a new image snapshot to ec2"
          end

          puts "Importing image from S3 path #{s3_url}"
          task_id_output = sh! "aws", "ec2", "import-snapshot",
            "--region", region,
            "--description", "#{name}",
            "--disk-container", {
            "Description" => "nixos-image-#{label}-#{system}",
            "Format"      => "vhd",
            "UserBucket"  => {
              "S3Bucket" => BUCKET,
              "S3Key"    => s3_name,
            },
          }.to_json
          puts
          if task_id = task_id_output
            image.task_id = ImportResult.from_json(task_id).import_task_id
          end
        end
      end

      def check_for_s3image(region, force)
        puts
        if !force
          puts "Checking for image on S3"
          return if sh "aws", "s3", "ls", "--region", BUCKET_REGION, s3_url
          puts
          puts "Image missing from aws s3, uploading"
        else
          puts "Force pushing the image to aws s3, uploading"
        end
        sh "aws", "s3", "cp", "--region", BUCKET_REGION, file, s3_url
      end

      def copy_to_region(region, from_region, from_ami_id, force)
        with_image region do |image|
          copy_to_region(region, from_region, from_ami_id, force, image)
        end
      end

      def copy_to_region(region, from_region, from_ami_id, force, image)
        if image.ami_id && !force
          if validate_ami_by_id(image.ami_id, region)
            puts "Using an existing AMI for #{region} in the registry file: #{image.ami_id}"
            return image.ami_id
          end
        end

        if force
          puts "Forcing an ami image copy to region #{region}"
        else
          output = sh "aws", "ec2", "describe-images",
            "--region", region,
            "--filters", "Name=name,Values=\"#{name}\""

          # Check for an existing ami image with the hash name; use it if it exists
          images = Hash(String, Array(ImageDescription)).from_json(output.to_s)
          images_match = images["Images"]
          if images_match.size >= 1 && images_match.first.state != "failed"
            if images_match.size == 1
              puts "Found an existing ami image with the name hash in the region #{region}.  " \
                   "Using the existing ami image: #{images_match.first.image_id}"
            else
              puts "Warning: multiple ami images with the name hash found in the region #{region}.  " \
                   "Using the first ami image found: #{images_match.first.image_id}"
            end
            image.ami_id = images_match.first.image_id
            puts
            return image.ami_id
          end
        end

        # Register a new ami in the copy-to region
        ami_id_output = sh! "aws", "ec2", "copy-image",
          "--region", region,
          "--source-region", from_region,
          "--source-image-id", from_ami_id,
          "--name", name,
          "--description", description
        puts "Created AMI ID #{image.ami_id} in region #{region}"
        image.ami_id = RegisterImageResult.from_json(ami_id_output).image_id
        puts

        # Find the new amis backing snapshot in the remote region
        puts "Checking for the remote backing snapshot"
        matches = 0
        checks = 0

        while matches == 0 && checks < 30
          sleep(1)
          existing_snapshot = shSilent!("aws", "ec2", "describe-snapshots",
            "--region", region,
            "--filters", "Name=description,Values=\"Copied for DestinationAmi " \
                         "#{image.ami_id.to_s.strip} from SourceAmi #{from_ami_id.to_s.strip}*\"")
          snapshot_match = SnapshotDescribe.from_json(existing_snapshot).matches
          checks += 1
          matches = snapshot_match.size
        end
        if matches == 0
          puts "Unable to find and tag the remote backing snapshot"
          remote_snapshot_id = "undefined"
        elsif matches == 1
          if extract_not_nil = snapshot_match
            remote_snapshot_id = extract_not_nil.first.snapshotId
            puts "Remote snapshot id: #{remote_snapshot_id}"
            puts
          end

          # Create tags on the new remote backing snapshot
          if sh!("aws", "ec2", "create-tags",
               "--region", region,
               "--resources", "#{remote_snapshot_id}",
               "--tags", "Key=Name,Value=\"#{name}\"",
               "Key=perf-ops,Value=\"#{perf_uuid}\"",
               "Key=source-region,Value=\"#{from_region}\"",
               "Key=ami,Value=\"#{image.ami_id}\"")
            puts "Snapshot tagging successfully created for the remote backing snapshot"
          else
            puts "Snapshot tagging of the remote backing snapshot failed."
          end
        else
          puts "An unexpected number of snapshot matches were found due to an unknown error"
        end
        puts

        # Create tags on the new remote ami
        if sh!("aws", "ec2", "create-tags",
             "--region", region,
             "--resources", "#{image.ami_id}",
             "--tags", "Key=Name,Value=\"#{name}\"",
             "Key=perf-ops,Value=\"#{perf_uuid}\"",
             "Key=source-region,Value=\"#{from_region}\"",
             "Key=snapshot-id,Value=\"#{remote_snapshot_id}\"")
          puts "Ami tag successfully created"
        else
          puts "Ami tagging of copied ami failed."
        end
        puts

        image.ami_id
      end

      class RegisterImageResult
        include JSON::Serializable

        @[JSON::Field(key: "ImageId")]
        property image_id : String
      end

      class SnapshotDescribe
        include JSON::Serializable

        @[JSON::Field(key: "Snapshots")]
        property matches : Array(SnapshotResults)

        class SnapshotResults
          include JSON::Serializable

          @[JSON::Field(key: "State")]
          property state : String

          @[JSON::Field(key: "SnapshotId")]
          property snapshot_id : String
        end
      end

      class ImportResult
        include JSON::Serializable

        @[JSON::Field(key: "ImportTaskId")]
        property import_task_id : String
      end

      class ImportSnapshotTasks
        include JSON::Serializable

        @[JSON::Field(key: "ImportSnapshotTasks")]
        property import_snapshot_tasks : Array(SnapshotTask)

        class SnapshotTaskDetail
          include JSON::Serializable

          @[JSON::Field(key: "Status")]
          property status : String

          @[JSON::Field(key: "Progress")]
          property progress : String?

          @[JSON::Field(key: "StatusMessage")]
          property status_message : String?

          @[JSON::Field(key: "SnapshotId")]
          property snapshot_id : String?
        end

        class SnapshotTask
          include JSON::Serializable

          @[JSON::Field(key: "SnapshotTaskDetail")]
          property snapshot_task_detail : SnapshotTaskDetail
        end
      end
    end
  end
end
