require "admiral"
require "json"
require "./ami-upload/**"

module Bitte
  class CLI < Admiral::Command
    register_sub_command ami : AMI, description: "manage AMIs"

    class AMI < Admiral::Command
      define_help short: "h", description: "manage AMIs"

      ALL_REGIONS = %w[
        eu-west-1 eu-west-2 eu-west-3 eu-central-1
        us-east-1 us-east-2 us-west-1 us-west-2
        ca-central-1
        ap-southeast-1 ap-southeast-2 ap-northeast-1 ap-northeast-2
        ap-south-1
        sa-east-1
      ]

      define_flag names : Array(String), short: "n",
        description: "AMI name(s) of the nix AMI attr(s) in default.nix as a string, space delimited for multiple; " \
                     "defaults to .envrc $AMI_FILTER behavior if not declared"

      define_flag regions : Array(String), short: "r",
        description: "Region(s) for generated AMIs to be pushed to as a string, space delimited for multiple; " \
                     "defaults to deploy-config.nix region definitions if not declared"

      define_flag bucket : String, required: true,
        description: "The bucket where the images will be stored"

      define_flag bucket_region : String, required: true,
        description: "The region of the S3 bucket"

      define_flag home : String,
        description: "The home region to push AMI images to and then copy from into other target regions"

      register_sub_command sync : Sync,
        description: "sync AMIs"

      class Sync < Admiral::Command
        define_help short: "h", description: "sync AMIs"

        define_flag force : Bool, short: "f",
          description: "Force push files, images, snapshots and AMI registrations, even if they already exist."

        define_flag ami : String, required: true,
          description: "The AMI"

        def regions
          parent.flags.as(CLI::AMI::Flags).regions
        end

        def run
          pp! regions

          # validate_regions(regions[flags.ami])
          # image = ImageInfo.prepare(flags.ami)
          #
          # puts <<-INFO
          # Image Details:
          #   Name: #{image.name}
          #   Description: #{image.description}
          #   Size: #{image.logical_gigabytes}GB
          #   System: #{image.system}
          #   Amazon Arch: #{image.amazon_arch}
          # INFO
          #
          # image.upload_all!(home_region, regions[ami], force)
        end

        def region_amis
          regions = Hash(String, Array(String)).new

          amis.each do |ami|
            regions[ami] = flags.regions.sort
          end

          regions
        end
      end

      register_sub_command clean : Clean, description: "clean AMIs"

      class Clean < Admiral::Command
        include Bitte::AMI::Common

        define_help short: "h", description: "clean AMIs"

        define_flag delete : Bool, default: false,
          description: "Delete AMIs and snapshots used by this perf-ops deploy " \
                                                                "across all regions accessible with the current aws credentials"

        define_flag purge : Bool, default: false,
          description: "Delete AMIs and snapshots used by ALL perf-ops deploys across all regions " \
                                                               "accessible with the current aws credentials"

        define_flag s3purge : Bool, default: false,
          description: "Delete *.vhd image paths and files in the s3 bucket, " \
                                                                 "accessible with the current aws credentials, used to generate snapshots and AMIs"

        def run
          if s3_purge?
            s3delete
          elsif delete?
            delete
          else
            STDERR.puts "one of --delete, --purge, or --s3purge has to be set"
            exit 1
          end
        end

        def delete?
          flags.delete || flags.purge
        end

        def purge?
          flags.purge
        end

        def s3_purge?
          flags.s3purge
        end

        # Check all images for availablility prior to deleting
        def delete_check
          puts "Checking images and snapshots are available prior to deletion " \
               "to keep the operation as atomic as possible."

          used_regions = [] of String

          ALL_REGIONS.each do |region|
            client = AWS::Client.new(region)

            print "#{region}: "

            images =
              if purge?
                client.describe_images("tag-key": "perf-ops")
              else
                client.describe_images("tag:perf-ops": perf_uuid)
              end

            puts images.images.size

            images.images.each do |image|
              if image.state != "available"
                puts "\nImage #{image.image_id} is not yet available in region #{region}.  " \
                     "Try again in a few minutes."
                exit
              end

              if ebs_device = image.block_device_mappings.first.ebs
                unless ebs_device.snapshot_id
                  puts "\nImage #{image.image_id} does not have an ebs snapshot listed yet " \
                       "in region #{region}.  Try again in a few minutes."
                  exit
                end
              end

              used_regions << region
            end
          end

          used_regions
        end

        def delete
          used_regions = delete_check

          used_regions.each do |region|
            client = AWS::Client.new(region)

            images =
              if purge?
                puts "Deleting all perf-ops deploy amis and snapshots from region #{region}"
                client.describe_images("tag-key": "perf-ops")
              else
                puts "Deleting perf-ops deploy \"#{perf_uuid}\" amis and snapshots from region #{region}"
                client.describe_images("tag:perf-ops": perf_uuid)
              end

            images.images.each do |image|
              unless image.state == "available" && (ebs_device = image.block_device_mappings.first.ebs)
                puts "Cannot delete #{image.image_id} and related snapshot -- ami is not yet available"
                next
              end

              puts "  Deleting #{image.image_id} and backing snapshot #{ebs_device.snapshot_id}"
              client.deregister_image(image.image_id)
              client.delete_snapshot(ebs_device.snapshot_id)
            end
          end

          puts "Deleting the registry file to rebuild clean state on the next sync operation"
          File.delete("state.json") if File.exists?("state.json")
        end

        def s3delete
          puts "Deleting *.vhd image paths and files in the s3 bucket: " \
               "--region #{bucket_region} s3://#{bucket}"
          AWS::Client.new(bucket_region).s3_rm("s3://#{bucket}/nix/store", recursive: true)
        end
      end
    end
  end
end

%q(

# Parse the --names CLI option
puts
if !config.has_key? "names"
  puts "Using $AMI_FILTER env default for AMI names to build"
  nix_json = sh("nix-instantiate", "--json", "--strict", "--eval", "-E", "__attrNames (import ./.).amis")
  amis = Array(String).from_json(nix_json.to_s)
else
  puts "Using name arg from CLI for AMI attrs"
  amis = config["names"].split
end
puts amis

# Parse the --home CLI option
puts
if !config.has_key? "home_region"
  puts "Using default home region of:"
  home_region = HOME_REGION
else
  puts "Using \"home\" arg from CLI for home region:"
  home_region = config["home_region"]
end
puts home_region

# Parse the --regions CLI option
puts
if !config.has_key? "regions"
  puts "Using deploy-config.nix region definitions to push AMI targets"
  nix_json = sh("nix-instantiate", "--json", "--strict", "--eval", "-E",
    "let pkgs = import <nixpkgs> {}; lib = pkgs.lib; " \
    "in (import ./config.nix { inherit lib pkgs ; }).variable.usedRegions.default")
  regions = Hash(String, Array(String)).from_json(nix_json.to_s)
else
  puts "Using \"region\" arg from CLI for per-image AMI region targets"
  regions = Hash(String, Array(String)).new
  amis.each do |ami|
    regions[ami] = config["regions"].split
  end
end
puts regions

if force
  puts
  puts "Force push CLI option selected"
end

puts
puts "AMI images to build and push: #{amis}"
puts

puts "Building images..."
amis.each do |ami|
  validate_regions(regions[ami])
  image = ImageInfo.prepare(ami)

  puts <<-INFO

  Image Details:
    Name: #{image.name}
    Description: #{image.description}
    Size: #{image.logical_gigabytes}GB
    System: #{image.system}
    Amazon Arch: #{image.amazon_arch}
  INFO

  image.upload_all!(home_region, regions[ami], force)

  puts
end
        end
      end
    end
  end
end



HOME_REGION = "eu-central-1"
ALL_REGIONS = %w[
   eu-west-1 eu-west-2 eu-west-3 eu-central-1
   us-east-1 us-east-2 us-west-1 us-west-2
   ca-central-1
   ap-southeast-1 ap-southeast-2 ap-northeast-1 ap-northeast-2
   ap-south-1
   sa-east-1
]
BUCKET = "iohk-midnight-amis"
BUCKET_REGION = "eu-central-1"

module Common
  def perf_uuid
    file = "perf-uuid.txt"
    if !ENV["PERF_UUIDF"]?.nil?
      ENV["PERF_UUID"]
    else
      if File.exists?(file) && !File.empty?(file)
        File.read(file).strip
      else
        "undefined"
      end
    end
  end

  def validate_regions(regions)
    regions.each do |region|
      raise "Error: #{region} is not a valid region" \
        if !ALL_REGIONS.includes? region
    end
  end
end
include Common

module Shell
  def sh_silent!(cmd, *args)
    output = IO::Memory.new
    Process.run(cmd, args, output: output, error: STDERR).tap do |status|
      raise "#{cmd} #{args} failed" unless status.success?
    end
    output.to_s.strip
  end

  def sh!(cmd, *args)
    puts "$ #{cmd} #{args.to_a.join(" ")}"
    output = IO::Memory.new
    Process.run(cmd, args, output: output, error: STDERR).tap do |status|
      raise "#{cmd} #{args} failed" unless status.success?
    end
    output.to_s.strip
  end

  def sh(cmd, *args)
    puts "$ #{cmd} #{args.to_a.join(" ")}"
    output = IO::Memory.new
    status = Process.run(cmd, args, output: output, error: STDERR)
    output.to_s.strip if status.success?
  end

  def sh_silent(cmd, *args)
    output = IO::Memory.new
    status = Process.run(cmd, args, output: output, error: STDERR)
    output.to_s.strip if status.success?
  end
end
extend Shell

class ImageInfo

  include Common
  include Shell

  JSON.mapping(
    label: String,
    system: String,
    logical_bytes: String,
    file: String,
  )

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
    (regions - [ home_region ]).each do |region|
      copy_to_region region, home_region, home_image_id.not_nil!, force
    end
  end

  def s3_name
    file.lstrip("/")
  end

  def s3_url
    "s3://#{BUCKET}/#{s3_name}"
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
    output = shSilent "aws", "ec2", "describe-images",
      "--region", region,
      "--filters", "Name=image-id,Values=\"#{ami_id}\""
    images = Hash(String, Array(ImageDescription)).from_json(output.to_s)
    images["Images"].size == 1
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
        print ("%4s/100 : %s" % [task.progress, task.status_message]).ljust(60)+"\r"
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
      if image.ami_id && !force
        if validate_ami_by_id(image.ami_id, region)
          puts "Using an existing AMI for #{region} in the registry file: #{image.ami_id}"
          return image.ami_id
        end
      end

      if !force
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
      else
        puts "Forcing an ami image copy to region #{region}"
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
  end

  class RegisterImageResult
    JSON.mapping(
      image_id: {type: String, key: "ImageId"}
    )
  end

  class SnapshotDescribe
    class SnapshotResults
      JSON.mapping(
        state: { type: String, key: "State" },
        snapshotId: { type: String, key: "SnapshotId" }
      )
    end

    JSON.mapping(
      matches: {
        type: Array(SnapshotResults),
        key: "Snapshots"
      }
    )
  end

  class ImportResult
    JSON.mapping(
      import_task_id: {type: String, key: "ImportTaskId"}
    )
  end

  class ImportSnapshotTasks
    class SnapshotTaskDetail
      JSON.mapping(
        status: {type: String, key: "Status"},
        progress: {type: String?, key: "Progress"},
        status_message: {type: String?, key: "StatusMessage"},
        snapshot_id: {type: String?, key: "SnapshotId"}
      )
    end

    class SnapshotTask
      JSON.mapping(
        snapshot_task_detail: {
          type: SnapshotTaskDetail,
          key:  "SnapshotTaskDetail",
        }
      )
    end

    JSON.mapping(
      import_snapshot_tasks: {
        type: Array(SnapshotTask),
        key:  "ImportSnapshotTasks",
      }
    )
  end
end

class Registry
  class Images
    JSON.mapping(images: Hash(String, State))

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
    JSON.mapping(
      task_id: String?,
      snapshot_id: String?,
      ami_id: String?
    )

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

class ImageDescription
  class EbsDetails
    include JSON::Serializable

    @[JSON::Field(key: "SnapshotId")]
    property snapshot_id : String?

    @[JSON::Field(key: "DeleteOnTermination")]
    property delete_on_termination : Bool?

    @[JSON::Field(key: "VolumeType")]
    property volume_type : String?

    @[JSON::Field(key: "VolumeSize")]
    property volume_size : Int32?

    @[JSON::Field(key: "Encrypted")]
    property encrypted : Bool?
  end

  class BlockDeviceMappings
    JSON.mapping(
        device_name: {type: String, key: "DeviceName"},
        ebs: {type: EbsDetails?, key: "Ebs"}
    )
  end

  JSON.mapping(
    virtualization_type: {type: String, key: "VirtualizationType"},
    description: {type: String, key: "Description"},
    hypervisor: {type: String, key: "Hypervisor"},
    ena_support: {type: Bool, key: "EnaSupport"},
    sriov_net_support: {type: String, key: "SriovNetSupport"},
    image_id: {type: String, key: "ImageId"},
    state: {type: String, key: "State"},
    architecture: {type: String, key: "Architecture"},
    image_location: {type: String, key: "ImageLocation"},
    root_device_type: {type: String, key: "RootDeviceType"},
    block_device_mappings: {type: Array(BlockDeviceMappings), key: "BlockDeviceMappings"},
    owner_id: {type: String, key: "OwnerId"},
    root_device_name: {type: String, key: "RootDeviceName"},
    creation_date: {type: String, key: "CreationDate"},
    public: {type: Bool, key: "Public"},
    image_type: {type: String, key: "ImageType"},
    name: {type: String, key: "Name"}
  )

  def self.s3delete
    puts "Deleting *.vhd image paths and files in the s3 bucket: " \
         "--region #{BUCKET_REGION} s3://#{BUCKET}"
    output = IO::Memory.new
    Process.run("aws",
      ["s3", "rm",
       "--region", BUCKET_REGION,
       "--recursive",
       "s3://#{BUCKET}/nix/store"
      ], output: output, error: STDERR
    )
  end

  def self.delete(purge)
    # Check all images for availablility prior to deleting
    print "Checking images and snapshots are available prior to deletion " \
          "to keep the operation as atomic as possible."
    ALL_REGIONS.each do |region|
      print "."
      output = IO::Memory.new
      if purge
        Process.run("aws",
          ["ec2", "describe-images",
           "--region", region,
           "--filters", "Name=tag-key,Values=\"perf-ops\"",
          ], output: output, error: STDERR
        )
      else
        Process.run("aws",
          ["ec2", "describe-images",
           "--region", region,
           "--filters", "Name=tag:perf-ops,Values=\"#{perf_uuid}\"",
          ], output: output, error: STDERR
        )
      end

      images = Hash(String, Array(AWS::Types::Images::ImageDescription)).from_json(output.to_s)
      images["Images"].each do |image|
        if image.state != "available"
          puts "\nImage #{image.image_id} is not yet available in region #{region}.  " \
               "Try again in a few minutes."
          exit(0)
        end
        if ebs_device = image.block_device_mappings.first.ebs
          snapshot_id = ebs_device.snapshot_id
          if snapshot_id.nil?
            puts "\nImage #{image.image_id} does not have an ebs snapshot listed yet " \
                 "in region #{region}.  Try again in a few minutes."
            exit(0)
          end
        end
      end
    end
    puts

    ALL_REGIONS.each do |region|
      output = IO::Memory.new
      if purge
        puts "Deleting all perf-ops deploy amis and snapshots from region #{region}"
        Process.run("aws",
          ["ec2", "describe-images",
           "--region", region,
           "--filters", "Name=tag-key,Values=\"perf-ops\"",
          ], output: output, error: STDERR
        )
      else
        puts "Deleting perf-ops deploy \"#{perf_uuid}\" amis and snapshots from region #{region}"
        Process.run("aws",
          ["ec2", "describe-images",
           "--region", region,
           "--filters", "Name=tag:perf-ops,Values=\"#{perf_uuid}\"",
          ], output: output, error: STDERR
        )
      end

      images = Hash(String, Array(ImageDescription)).from_json(output.to_s)

      images["Images"].each do |image|
        if image.state == "available" && (ebs_device = image.block_device_mappings.first.ebs)
          puts "  Deleting #{image.image_id} and backing snapshot #{ebs_device.snapshot_id}"
          Process.run("aws",
            ["ec2", "deregister-image",
             "--image-id", image.image_id,
             "--region", region,
            ], error: STDERR, output: STDOUT)
          Process.run("aws",
            ["ec2", "delete-snapshot",
             "--region", region,
             "--snapshot-id", "#{ebs_device.snapshot_id}",
            ], output: output, error: STDERR
          )
        else
          puts "Cannot delete #{image.image_id} and related snapshot -- ami is not yet available"
        end
      end
    end
    puts "Deleting the registry file to rebuild clean state on the next sync operation"
    File.delete("state.json") if File.exists?("state.json")
  end

  def self.deregister_all
    ALL_REGIONS.each do |region|
      output = IO::Memory.new
      Process.run("aws",
        ["ec2", "describe-images",
         "--region", region,
         "--filters", "Name=description,Values=\"NixOS 19.09pre-git x86_64-linux\"",
        ], output: output, error: STDERR
      )

      images = Hash(String, Array(ImageDescription)).from_json(output.to_s)

      images["Images"].each do |image|
        puts "deregister #{image.image_id} in #{region}"
        Process.run("aws",
          ["ec2", "deregister-image",
           "--image-id", image.image_id,
           "--region", region,
          ], error: STDERR, output: STDOUT)
      end
    end
  end
end
)
