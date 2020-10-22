mod sensors;
use sensors::{Topology, Sensor, powercap_rapl::PowercapRAPLSensor};

pub fn run() {
    let mut sensor = PowercapRAPLSensor::new();
    let mut topology = *sensor.get_topology();
    let topology = match topology {
        Some(topo) => topo,
        None => panic!("Topology has not been generated.")
    };
    println!("topology: {:?}\n\n\n\n", topology);
    for socket in &topology.sockets {
        println!("Browsing socket {}", socket.id);
        println!(
            "Overall socket energy: {} µJ",
            socket.read_counter_uj().unwrap()
        );
        for domain in &socket.domains {
            println!("Browsing domain {} : {}", domain.id, domain.name);
            println!(
                "Current energy counter value: {} µJ",
                domain.read_counter_uj().unwrap()
            );
        }
    }
}

//fn get_str_from_dir_entry(e: &fs::DirEntry) -> String{
//    String::from(e.path().as_path().to_str().unwrap())
//}

//pub fn raw_read_from_powercap_sysfs() -> Result<String, errors::PowercapReadError>{
//    let base_path = "/sys/class/powercap";
//    let folders = fs::read_dir(&base_path).unwrap();
//    let re = Regex::new(r"intel-rapl:\d{1}:\d{1}").unwrap();
//    let mut result = String::from("");
//    for i in folders {
//        let i = i.unwrap();
//        let istr = get_str_from_dir_entry(&i);
//        if re.is_match(&istr){                               
//            let energy = fs::read_to_string(format!("{}/{}", istr, "energy_uj")).unwrap();
//            println!("energy: {}", energy);
//            let unit = Domain::new_from_name(&fs::read_to_string(format!("{}/{}", istr, "name")).unwrap().trim());            
//            let value: u128 = energy.trim().parse::<u128>().unwrap();
//            let record = Record {
//                unit,
//                value: value
//            };
//            
//            result.push_str(&record.to_string());
//            result.push_str("\n");
//
//        }
//    }
//    Ok(result)
//}
