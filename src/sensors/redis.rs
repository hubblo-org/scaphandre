use crate::exporters::qemu_redis::QemuRedisMetric;
use crate::sensors::{Sensor, Topology};
use log::{debug, error};
use redis::{Client};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration};

/// A sensor that subscribes to Redis topics to receive QEMU VM energy consumption data
pub struct RedisSensor {
    redis_url: String,
    redis_prefix: String,
    vm_name: String,
    running: Arc<Mutex<bool>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VmEnergyData {
    vm_name: String,
    energy_uj: u64,
    timestamp: u64,
}

impl RedisSensor {
    /// Creates a new RedisSensor instance
    pub fn new(redis_url: &str, redis_prefix: &str, vm_name: &str) -> RedisSensor {
        RedisSensor {
            redis_url: redis_url.to_string(),
            redis_prefix: redis_prefix.to_string(),
            vm_name: vm_name.to_string(),
            running: Arc::new(Mutex::new(true)),
        }
    }

    /// Starts the Redis subscription thread
    fn start_subscription_thread(&self) {
        let client = Client::open(self.redis_url.clone()).unwrap();
        let redis_prefix = self.redis_prefix.clone();
        let vm_name = self.vm_name.clone();
        let running = Arc::clone(&self.running);

        thread::spawn(move || {
            let mut connection = client.get_connection().expect("Failed to connect to Redis");

            let mut pubsub = connection.as_pubsub();
            let topic = format!("{}:{}", redis_prefix, vm_name);
            println!("Subscribing to Redis topic: {}", topic);
            pubsub.subscribe(topic).unwrap();

            while *running.lock().unwrap() {
                let msg = pubsub.get_message();
                match msg {
                    Ok(msg) => {
                        let payload: String = msg.get_payload().unwrap_or_default();
                        let channel: String = msg.get_channel_name().to_string();

                        debug!("Received message on channel {}: {}", channel, payload);
                        // deserialize the payload to a QemuRedisMetric
                        let metric: QemuRedisMetric = serde_json::from_str(&payload)
                            .unwrap_or_else(|e| {
                                error!("Failed to deserialize message: {}", e);
                                QemuRedisMetric {
                                    vm_name: String::new(),
                                    energy_uj: 0.0,
                                    timestamp: 0,
                                }
                            });

                        let dir = "/tmp";
                        let file_path = format!("{}/vm_energy_uj", dir);
                        fs::write(file_path, (metric.energy_uj as u64).to_string()).unwrap_or_else(|e| {
                            error!("Failed to write VM data to file: {}", e);
                        });
                    }
                    Err(e) => {
                        error!("Error receiving Redis message: {}", e);
                        thread::sleep(Duration::from_secs(1));
                    }
                }
            }
        });
    }

    /// Stops the Redis subscription thread
    pub fn stop(&self) {
        let mut running = self.running.lock().unwrap();
        *running = false;
    }
}

impl Sensor for RedisSensor {
    /// Creates a Topology instance.
    fn generate_topology(&self) -> Result<Topology, Box<dyn Error>> {
        println!("Starting subscription thread");
        self.start_subscription_thread();

        let mut topo = Topology::new(HashMap::new(), true);
        let mut sensor_data_for_socket = HashMap::new();

        let socket_id = 0; // Assuming a single socket for simplicity
        let base_path = "/tmp";
        sensor_data_for_socket.insert(
            String::from("source_file"),
            format!("{}/vm_energy_uj", base_path),
        );
        topo.safe_add_socket(
            socket_id,
            vec![],
            vec![],
            format!("{}/vm_energy_uj", base_path),
            1,
            sensor_data_for_socket,
        );

        topo.add_cpu_cores();
        Ok(topo)
    }

    /// Instanciates Topology object if not existing and returns it
    fn get_topology(&self) -> Box<Option<Topology>> {
        let topology = self.generate_topology().ok();
        if topology.is_none() {
            panic!("Couldn't generate the topology !");
        }
        Box::new(topology)
    }
}
