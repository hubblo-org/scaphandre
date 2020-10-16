
use std::fs;
use regex::Regex;
use std::fmt;

mod errors;
mod tests;

enum UnitKind {
    Core,
    Uncore,
    Dram,
    Undefined
}


impl fmt::Display for UnitKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            UnitKind::Core => "core",
            UnitKind::Uncore => "uncore",
            UnitKind::Dram => "dram",
            _ => "undefined"
        };
        write!(f, "{}", name)
    }
}

struct Unit {
    kind: UnitKind,
}

impl Unit {
    fn new_from_name (name: &str) -> Unit{
       let kind = match name {
           "core" => UnitKind::Core,
           "uncore" => UnitKind::Uncore,
           "dram" => UnitKind::Dram,
           _ => UnitKind::Undefined
       };
       Unit { kind: kind }
    }   
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

struct Record {
    unit: Unit,
    value: u128 
}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "recorded {} µJoules on device unit {}", self.value, self.unit)
    }
}


fn get_str_from_dir_entry(e: &fs::DirEntry) -> String{
    String::from(e.path().as_path().to_str().unwrap())
}

pub fn raw_read_from_powercap_sysfs() -> Result<String, errors::PowercapReadError>{
    let base_path = "/sys/class/powercap";
    let folders = fs::read_dir(&base_path).unwrap();
    let re = Regex::new(r"intel-rapl:\d{1}:\d{1}").unwrap();
    let mut result = String::from("");
    for i in folders {
        let i = i.unwrap();
        let istr = get_str_from_dir_entry(&i);
        if re.is_match(&istr){                               
            let energy = fs::read_to_string(format!("{}/{}", istr, "energy_uj")).unwrap();
            println!("energy: {}", energy);
            let unit = Unit::new_from_name(&fs::read_to_string(format!("{}/{}", istr, "name")).unwrap().trim());            
            let value: u128 = energy.trim().parse::<u128>().unwrap();
            let record = Record {
                unit,
                value: value
            };
            
            result.push_str(&record.to_string());
            result.push_str("\n");

        }
    }
    Ok(result)
}