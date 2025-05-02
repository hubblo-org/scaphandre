# Qemu Redis exporter

Computes energy consumption metrics for each Qemu/KVM virtual machine found on the host.
Publish the metrics on a Redis server from which the Scaphandre instance running on the VM can read them.

## Usage

1. Run Scaphandre with the qemu exporter on your bare metal hypervisor machine:
    ```bash
    scaphandre qemu-redis --redis-url 'redis://192.168.254.94/' --redis_prefix 'hypervisor1' -s 1
    ```
    
    The `--redis-url` option is mandatory. It specifies the Redis server to which the metrics will be published.
    
    The `--redis_prefix` option is optional and allows you to specify a prefix for the topic on Redis, this is useful if you want to use the same Redis instance for multiple servers, especially if you have a hypervisor with multiple nodes.
    
    The `-s` option is the sampling interval in seconds. The default value is 2 seconds, but you can set it to a higher value if you want to reduce the load.
      
2. Run Scaphandre with the redis sensor on your VM: [redis sensor](sensor-redis.md).