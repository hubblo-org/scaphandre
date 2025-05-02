# Redis sensor

To read the metrics published by the Qemu Redis exporter, you need to run Scaphandre with the redis sensor on your VM.

## Usage
You can run the redis sensor this way:
```bash
scaphandre --sensor redis --redis-url 'redis://192.168.254.94/' --redis-prefix "hypervisor1" --vm-name "instance-000001be" --vm stdout -t 1000 -s 5
```

The `--redis-url` option is mandatory. It specifies the Redis server from which the metrics will be read.

The `--redis-prefix` option is optional and allows you to specify a prefix for the topic on Redis, this is useful if you want to use the same Redis instance for multiple servers, especially if you have a hypervisor with multiple nodes.

The `--vm-name` option is mandatory. It specifies the name of the VM for which you want to read the metrics, this should be the name of the VM on which you are running this instance of Scaphandre.

