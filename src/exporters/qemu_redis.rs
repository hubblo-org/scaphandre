use crate::exporters::Exporter;
use crate::sensors::Topology;
use crate::sensors::{Sensor};
use log::info;
use redis::{Client, Commands, Connection, ConnectionLike};
use std::collections::HashMap;
use std::{thread, time};
use crate::exporters::utils::{filter_qemu_vm_processes, get_vm_name_from_cmdline};

/// An Exporter that publishes QEMU VM energy consumption data to Redis
pub struct QemuRedisExporter {
    topology: Topology,
    redis_client: Client,
    redis_connection: Option<Connection>,
    args: QemuRedisExporterArgs,
    energy_accumulator: HashMap<String, f64>,
}

#[derive(clap::Args, Debug)]
pub struct QemuRedisExporterArgs {
    /// Redis server URL
    #[arg(long, default_value = "redis://127.0.0.1/")]
    pub redis_url: String,

    /// Prefix for Redis keys
    #[arg(long, default_value = "scaphandre")]
    pub redis_prefix: String,

    /// Interval between two measurements, in seconds
    #[arg(short, long, value_name = "SECONDS", default_value_t = 2)]
    pub step: u64,
}


#[derive(serde::Serialize, serde::Deserialize)]
pub struct QemuRedisMetric {
    pub(crate) vm_name: String,
    pub(crate) energy_uj: f64,
    pub(crate) timestamp: i64,
}

impl Exporter for QemuRedisExporter {
    fn run(&mut self) {
        info!("Starting Redis exporter");
        let cleaner_step = 120;
        let mut timer = time::Duration::from_secs(cleaner_step);

        loop {
            self.iterate();
            let step = time::Duration::from_secs(self.args.step);
            thread::sleep(step);
            if timer - step > time::Duration::from_millis(0) {
                timer -= step;
            } else {
                self.topology
                    .proc_tracker
                    .clean_terminated_process_records_vectors();
                timer = time::Duration::from_secs(cleaner_step);
            }
        }
    }

    fn kind(&self) -> &str {
        "qemu_redis"
    }
}

impl QemuRedisExporter {
    /// Creates a new RedisExporter instance
    pub fn new(sensor: &dyn Sensor, args: QemuRedisExporterArgs) -> QemuRedisExporter {
        let topology = sensor
            .get_topology()
            .expect("sensor topology should be available");

        QemuRedisExporter {
            topology,
            redis_client: Client::open(args.redis_url.clone()).unwrap(),
            redis_connection: None,
            args,
            energy_accumulator: HashMap::new(),
        }
    }

    pub fn iterate(&mut self) {
        self.topology.refresh();
        if let Some(topo_energy) = self.topology.get_records_diff_power_microwatts() {
            let processes = self.topology.proc_tracker.get_alive_processes();
            let qemu_processes = filter_qemu_vm_processes(&processes);

            if qemu_processes.is_empty() {
                return;
            }
            
            if self.redis_connection.is_none() {
                info!("Redis connection is None, trying to establish a new connection");
                match self.redis_client.get_connection() {
                    Ok(conn) => {
                        self.redis_connection = Some(conn);
                    }
                    Err(e) => {
                        eprintln!("Failed to get Redis connection: {}", e);
                        return;
                    }
                }
            } else { 
                info!("Redis connection is Some, checking if still up");
                if let Some(conn) = &mut self.redis_connection {
                    if !conn.check_connection() {
                        eprintln!("Redis connection is not valid");
                        self.redis_connection = None;
                        return;
                    }
                }
            }

            for qp in qemu_processes {
                if qp.len() > 2 {
                    let last = qp.first().unwrap();
                    let vm_name = get_vm_name_from_cmdline(
                        &last.process.cmdline(&self.topology.proc_tracker).unwrap(),
                    );

                    if let Some(ratio) = self
                        .topology
                        .get_process_cpu_usage_percentage(last.process.pid)
                    {
                        let energy_uj = ratio.value.parse::<f64>().unwrap()
                            * topo_energy.value.parse::<f64>().unwrap()
                            / 100.0;

                        // Accumulate power for the VM
                        self.energy_accumulator
                            .entry(vm_name.clone())
                            .and_modify(|acc| *acc += energy_uj)
                            .or_insert(0.0);
                        let acc_energy = *self.energy_accumulator.get(&vm_name).unwrap();
                        debug!("VM {} additional {} uJ since last measure, total: {} uJ", vm_name, energy_uj, acc_energy);

                        let metric = QemuRedisMetric {
                            vm_name: vm_name.clone(),
                            energy_uj: acc_energy,
                            timestamp: chrono::Utc::now().timestamp(),
                        };

                        match serde_json::to_string(&metric) {
                            Ok(json) => {
                                let data = json.into_bytes();
                                
                                let conn = self.redis_connection.as_mut().unwrap();

                                match conn.publish::<String, Vec<u8>, usize>(
                                    format!("{}:{}", self.args.redis_prefix, vm_name),
                                    data,
                                ) {
                                    Ok(_) => {
                                        info!("Published metric to Redis for VM: {}", vm_name);
                                    }
                                    Err(e) => {
                                        error!("Failed to publish metric to Redis: {}", e);
                                    }
                                }
                            }
                            Err(e) => error!("Failed to serialize metric for {}: {}", vm_name, e),
                        }
                    }
                }
            }
        }
    }
}
