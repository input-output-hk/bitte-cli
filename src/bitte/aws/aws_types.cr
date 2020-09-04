require "json"

# AWS API needs mapping for _all_the_things_
module AWS::Types
  module KMS
    class ListKeys
      include JSON::Serializable

      @[JSON::Field(key: "Keys")]
      property keys : Array(Key)
    end

    class Key
      include JSON::Serializable

      @[JSON::Field(key: "KeyId")]
      property key_id : String

      @[JSON::Field(key: "KeyArn")]
      property key_arn : String
    end
  end

  module Autoscaling
    class DescribeAutoScalingGroups
      include JSON::Serializable

      @[JSON::Field(key: "AutoScalingGroups")]
      property auto_scaling_groups : Array(AutoScalingGroup)
    end

    class AutoScalingGroup
      include JSON::Serializable

      @[JSON::Field(key: "AutoScalingGroupARN")]
      property arn : String

      @[JSON::Field(key: "Instances")]
      property instances : Array(Instance)

      @[JSON::Field(key: "Tags")]
      property tags : Array(Tag)
    end

    class Tag
      include JSON::Serializable

      @[JSON::Field(key: "Key")]
      property key : String

      @[JSON::Field(key: "Value")]
      property value : String
    end

    class Instance
      include JSON::Serializable

      @[JSON::Field(key: "InstanceId")]
      property instance_id : String

      @[JSON::Field(key: "InstanceType")]
      property instance_type : String

      @[JSON::Field(key: "AvailabilityZone")]
      property availability_zone : String

      @[JSON::Field(key: "LifecycleState")]
      property lifecycle_state : String

      @[JSON::Field(key: "HealthStatus")]
      property health_status : String

      @[JSON::Field(key: "LaunchConfigurationName")]
      property launch_configuration_name : String?

      @[JSON::Field(key: "ProtectedFromScaleIn")]
      property protected_from_scale_in : Bool
    end
  end

  module EC2
    class DescribeInstances
      include JSON::Serializable

      @[JSON::Field(key: "Reservations")]
      property reservations : Array(Reservation)
    end

    class Reservation
      include JSON::Serializable

      @[JSON::Field(key: "Instances")]
      property instances : Array(Instance)
    end

    class Instance
      include JSON::Serializable

      @[JSON::Field(key: "AmiLaunchIndex")]
      property ami_launch_index : Int32

      @[JSON::Field(key: "ImageId")]
      property image_id : String

      @[JSON::Field(key: "InstanceId")]
      property instance_id : String

      @[JSON::Field(key: "InstanceType")]
      property instance_type : String

      @[JSON::Field(key: "LaunchTime")]
      property launch_time : String

      @[JSON::Field(key: "Monitoring")]
      property monitoring : Monitoring

      @[JSON::Field(key: "Placement")]
      property placement : Placement

      @[JSON::Field(key: "PrivateIpAddress")]
      property private_ip_address : String?

      @[JSON::Field(key: "ProductCodes")]
      property product_codes : Array(JSON::Any?)

      @[JSON::Field(key: "PublicDnsName")]
      property public_dns_name : String

      @[JSON::Field(key: "PublicIpAddress")]
      property public_ip_address : String?

      @[JSON::Field(key: "State")]
      property state : State

      @[JSON::Field(key: "Tags")]
      property tags : Array(Tag)?

      def tags_hash
        if t = tags
          t.map { |tag| [tag.key, tag.value] }.to_h
        else
          Hash(String, String).new
        end
      end
    end

    class Monitoring
      include JSON::Serializable

      @[JSON::Field(key: "State")]
      property state : String
    end

    class Placement
      include JSON::Serializable

      @[JSON::Field(key: "AvailabilityZone")]
      property availability_zone : String
    end

    class State
      include JSON::Serializable

      @[JSON::Field(key: "Name")]
      property name : String
    end

    class Tag
      include JSON::Serializable

      @[JSON::Field(key: "Key")]
      property key : String

      @[JSON::Field(key: "Value")]
      property value : String
    end
  end

  module DescribeImages
    class Images
      include JSON::Serializable

      @[JSON::Field(key: "Images")]
      property images : Array(Image)
    end

    class Image
      include JSON::Serializable

      @[JSON::Field(key: "Architecture")]
      property architecture : String

      @[JSON::Field(key: "CreationDate")]
      property creation_date : String

      @[JSON::Field(key: "ImageId")]
      property image_id : String

      @[JSON::Field(key: "ImageLocation")]
      property image_location : String

      @[JSON::Field(key: "ImageType")]
      property image_type : String

      @[JSON::Field(key: "Public")]
      property public : Bool

      @[JSON::Field(key: "OwnerId")]
      property owner_id : String

      @[JSON::Field(key: "Platform")]
      property platform : String

      @[JSON::Field(key: "PlatformDetails")]
      property platform_details : String

      @[JSON::Field(key: "UsageOperation")]
      property usage_operation : String

      @[JSON::Field(key: "State")]
      property state : String

      @[JSON::Field(key: "BlockDeviceMappings")]
      property block_device_mappings : Array(BlockDeviceMapping)

      @[JSON::Field(key: "EnaSupport")]
      property ena_support : Bool

      @[JSON::Field(key: "Hypervisor")]
      property hypervisor : String

      @[JSON::Field(key: "ImageOwnerAlias")]
      property image_owner_alias : String

      @[JSON::Field(key: "Name")]
      property name : String

      @[JSON::Field(key: "RootDeviceName")]
      property root_device_name : String

      @[JSON::Field(key: "RootDeviceType")]
      property root_device_type : String

      @[JSON::Field(key: "SriovNetSupport")]
      property sriov_net_support : String

      @[JSON::Field(key: "VirtualizationType")]
      property virtualization_type : String

      @[JSON::Field(key: "Description")]
      property description : String?
    end

    class BlockDeviceMapping
      include JSON::Serializable

      @[JSON::Field(key: "DeviceName")]
      property device_name : String

      @[JSON::Field(key: "Ebs")]
      property ebs : Ebs?

      @[JSON::Field(key: "VirtualName")]
      property virtual_name : String?
    end

    class Ebs
      include JSON::Serializable

      @[JSON::Field(key: "DeleteOnTermination")]
      property delete_on_termination : Bool

      @[JSON::Field(key: "SnapshotId")]
      property snapshot_id : String

      @[JSON::Field(key: "VolumeSize")]
      property volume_size : Int32

      @[JSON::Field(key: "VolumeType")]
      property volume_type : String

      @[JSON::Field(key: "Encrypted")]
      property encrypted : Bool
    end
  end
end
