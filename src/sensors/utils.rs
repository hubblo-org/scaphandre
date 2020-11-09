use std::time::SystemTime;
use crate::sensors::units;
use crate::sensors::Record;

pub fn create_record_from_jiffries(value: String) -> Record {
    Record::new(
        SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap(),
        value,
        units::Unit::Jiffries
    )
}