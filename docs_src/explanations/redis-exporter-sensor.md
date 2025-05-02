# Explanation of Redis Exporter and Sensor
If it's not possible to use the normal Qemu exporter, you can use the Redis exporter and sensor to get the same metrics.

This is useful if you can't change the Qemu command line argument or the libvirt configuration, for example if you are using Openstack.

The Redis exporter runs on the hypervisor and publishes the metrics on a Redis server. The Redis sensor runs on the VM and reads the metrics from the Redis server.

## Redis topics and messages
The topic on which the metrics are published is `<redis_prefix>:<vm_name>`. An example of the JSON object that is published on this topic is:
```json
{
  "vm_name": "instance-000001be",
  "energy_uj": 1000.123,
  "timestamp": 1744965831
}
```

The `energy_uj` field is cumulative, the same as the file written by the Qemu exporter.