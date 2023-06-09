# Metrics exposed by Scaphandre

With [Stdout](exporter-stdout.md) exporter, you can see all metrics available on your machine with flag `--raw-metrics`.
With [prometheus](exporter-prometheus.md) exporter, all metrics have a HELP section provided on /metrics (or whatever suffix you choosed to expose them).

Here are some key metrics that you will most probably be interested in:

- `scaph_host_power_microwatts`: Aggregation of several measurements to give a try on the power usage of the the whole host, in microwatts (GAUGE). It might be the same as RAPL PSYS (see [RAPL domains](../explanations/rapl-domains.md)) measurement if available, or a combination of RAPL PKG and DRAM domains + an estimation of other hardware componentes power usage.
- `scaph_process_power_consumption_microwatts{exe="$PROCESS_EXE",pid="$PROCESS_PID",cmdline="path/to/exe --and-maybe-options"}`: Power consumption due to the process, measured on at the topology level, in microwatts. PROCESS_EXE being the name of the executable and PROCESS_PID being the pid of the process. (GAUGE)

For more details on that metric labels, see [this section](#getting-per-process-data-with-scaph_process_-metrics).

And some more deep metrics that you may want if you need to make more complex calculations and data processing:

- `scaph_host_energy_microjoules` : Energy measurement for the whole host, as extracted from the sensor, in microjoules. (COUNTER)
- `scaph_socket_power_microwatts{socket_id="$SOCKET_ID"}`: Power measurement relative to a CPU socket, in microwatts. SOCKET_ID being the socket numerical id (GAUGE)

If your machine provides RAPL PSYS domain (see [RAPL domains](../explanations/rapl-domains.md)), you can get the raw energy counter for PSYS/platform with `scaph_host_rapl_psys_microjoules`. Note that `scaph_host_power_microwatts` is based on this PSYS counter if it is available.

Since 1.0.0 the following host metrics are availalable as well ;

- `scaph_host_swap_total_bytes`: Total swap space on the host, in bytes.
- `scaph_host_swap_free_bytes`: Swap space free to be used on the host, in bytes.
- `scaph_host_memory_free_bytes`: Random Access Memory free to be used (not reused) on the host, in bytes.
- `scaph_host_memory_available_bytes`: Random Access Memory available to be re-used on the host, in bytes.
- `scaph_host_memory_total_bytes`: Random Access Memory installed on the host, in bytes.
- `scaph_host_disk_total_bytes`: Total disk size, in bytes.
- `scaph_host_disk_available_bytes`: Available disk space, in bytes.

Disk metrics have the following labels : disk_file_system, disk_is_removable, disk_type, disk_mount_point, disk_name

- `scaph_host_cpu_frequency`: Global frequency of all the cpus. In MegaHertz
- `scaph_host_load_avg_fifteen`: Load average on 15 minutes.
- `scaph_host_load_avg_five`: Load average on 5 minutes.
- `scaph_host_load_avg_one`: Load average on 1 minute.

If you hack scaph or just want to investigate its behavior, you may be interested in some internal metrics:

- `scaph_self_memory_bytes`: Scaphandre memory usage, in bytes

- `scaph_self_memory_virtual_bytes`: Scaphandre virtual memory usage, in bytes

- `scaph_self_topo_stats_nb`: Number of CPUStat traces stored for the host

- `scaph_self_topo_records_nb`: Number of energy consumption Records stored for the host

- `scaph_self_topo_procs_nb`: Number of processes monitored by scaph

- `scaph_self_socket_stats_nb{socket_id="SOCKET_ID"}`: Number of CPUStat traces stored for each socket

- `scaph_self_socket_records_nb{socket_id="SOCKET_ID"}`: Number of energy consumption Records stored for each socket, with SOCKET_ID being the id of the socket measured

- `scaph_self_domain_records_nb{socket_id="SOCKET_ID",rapl_domain_name="RAPL_DOMAIN_NAME
"}`: Number of energy consumption Records stored for a Domain, where SOCKET_ID identifies the socket and RAPL_DOMAIN_NAME identifies the rapl domain measured on that socket

### Getting per process data with scaph_process_* metrics

Here are available labels for the `scaph_process_power_consumption_microwatts` metric that you may need to extract the data you need:

- `exe`: is the name of the executable that is the origin of that process. This is good to be used when your application is running one or only a few processes.
- `cmdline`: this contains the whole command line with the executable path and its parameters (concatenated). You can filter on this label by using prometheus `=~` operator to match a regular expression pattern. This is very practical in many situations.
- `instance`: this is a prometheus generated label to enable you to filter the metrics by the originating host. This is very useful when you monitor distributed services, so that you can not only sum the metrics for the same service on the different hosts but also see what instance of that service is consuming the most, or notice differences beteween hosts that may not have the same hardware, and so on...
- `pid`: is the process id, which is useful if you want to track a specific process and have your eyes on what's happening on the host, but not so practical to use in a more general use case

Since 1.0.0 the following per-process metrics are available as well :

- `scaph_process_cpu_usage_percentage`: CPU time consumed by the process, as a percentage of the capacity of all the CPU Cores
- `scaph_process_memory_bytes`: Physical RAM usage by the process, in bytes
- `scaph_process_memory_virtual_bytes`: Virtual RAM usage by the process, in bytes
- `scaph_process_disk_total_write_bytes`: Total data written on disk by the process, in bytes
- `scaph_process_disk_write_bytes`: Data written on disk by the process, in bytes
- `scaph_process_disk_read_bytes`: Data read on disk by the process, in bytes
- `scaph_process_disk_total_read_bytes`: Total data read on disk by the process, in bytes

### Get container-specific labels on scaph_process_* metrics

The flag --containers enables Scaphandre to collect data about the running Docker containers or Kubernetes pods on the local machine. This way, it adds specific labels to make filtering processes power consumption metrics by their encapsulation in containers easier.

Generic labels help to identify the container runtime and scheduler used (based on the content of `/proc/PID/cgroup`):

`container_scheduler`: possible values are `docker` or `kubernetes`. If this label is not attached to the metric, it means that scaphandre didn't manage to identify the container scheduler based on cgroups data.

Then the label `container_runtime` could be attached. The only possible value for now is `containerd`.

`container_id` is the ID scaphandre got from /proc/PID/cgroup for that container.

For Docker containers (if `container_scheduler` is set), available labels are :

- `container_names`: is a string containing names attached to that container, according to the docker daemon
- `container_docker_version`: version of the docker daemon
- `container_label_maintainer`: content of the maintainer field for this container

For containers coming from a docker-compose file, there are a bunch of labels related to data coming from the docker daemon:

- `container_label_com_docker_compose_project_working_dir`
- `container_label_com_docker_compose_container_number`
- `container_label_com_docker_compose_project_config_files`
- `container_label_com_docker_compose_version`
- `container_label_com_docker_compose_service`
- `container_label_com_docker_compose_oneoff`

For Kubernetes pods (if `container_scheduler` is set), available labels are :

- `kubernetes_node_name`: identifies the name of the kubernetes node scaphandre is running on
- `kubernetes_pod_name`: the name of the pod the container belongs to
- `kubernetes_pod_namespace`: the namespace of the pod the container belongs to