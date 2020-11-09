use crate::sensors::units;
use crate::sensors::Record;
use procfs::process::Process;
use std::time::{Duration, SystemTime, SystemTimeError};

pub fn create_record_from_jiffies(value: String) -> Record {
    Record::new(
        SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap(),
        value,
        units::Unit::Jiffries
    )
}

#[derive(Debug)]
pub struct ProcessTracker {
    pub procs: Vec<Vec<ProcessRecord>>,
    pub max_records_per_process: u16
}

impl ProcessTracker {
    pub fn new(max_records_per_process: u16) -> ProcessTracker {
        ProcessTracker {
            procs: vec![],
            max_records_per_process
        }
    }

    /// Properly creates and adds a ProcessRecord to 'procs', the vector of vectors or ProcessRecords
    /// owned by the ProcessTracker instance. This method should be used to keep track of processes
    /// states during all the lifecycle of the exporter.
    pub fn add_process_record(&mut self, process: Process) -> Result<String, String> {
        let iterator = self.procs.iter_mut();
        let pid = process.pid;
        // find the vector containing Process instances with the same pid
        let mut filtered = iterator.filter(
            |x| x.len() > 0 && x[0].process.pid == pid
        );
        let result = filtered.next();
        let process_record = ProcessRecord::new(process);
        if result.is_some() { // if a vector of process records has been found
            let vector = result.unwrap();
            vector.push(process_record); // we add the process record to the vector
            if filtered.next().is_some() {
                panic!("Found more than one set of ProcessRecord (more than one pid) that matches the current process.");
            }
            ProcessTracker::clean_old_process_records(vector, self.max_records_per_process);
        } else { // if no vector of process records with the same pid has been found in self.procs
            self.procs.push(vec![process_record]); // we create a new vector in self.procs
        }

        Ok(String::from("Successfully added record to process."))
    }

    /// Removes as many ProcessRecords as needed from the vector (passed as a mutable ref in parameters)
    /// in order for the vector length to match self.max_records_per_process.
    fn clean_old_process_records(records: &mut Vec<ProcessRecord>, max_records_per_process: u16) {
        if records.len() > max_records_per_process as usize {
            let diff = records.len() - max_records_per_process as usize;
            for _ in 0..diff {
                records.sort_by(|a, b| {
                    //println!("{:?} {:?} {:?}", a.timestamp, b.timestamp, a.timestamp.cmp(&b.timestamp));
                    b.timestamp.cmp(&a.timestamp)
                });
                //println!("{:?}", records);
                //println!("i = {}", i as u16);
                //for r in records.iter() {
                //    println!("{:?}", r.timestamp);
                //}
                records.pop();
            }
        }
    }

    /// Returns a Some(ref to vector of ProcessRecords) if the pid is found
    /// in self.procs. Returns None otherwise.
    pub fn find_records(&self, pid: i32) -> Option<&Vec<ProcessRecord>> {
        for v in &self.procs {
            if v.len() > 0 && v[0].process.pid == pid {
                return Some(&v)
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct ProcessRecord {
    pub process: Process,
    pub timestamp: Duration
}

impl ProcessRecord {
    pub fn new(process: Process) -> ProcessRecord {
        ProcessRecord {
            process,
            timestamp: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap()
        }
    }
}

pub fn current_system_time_since_epoch() -> Result<Duration, SystemTimeError>{
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn process_records_added() {
        let proc = Process::myself().unwrap();
        let mut tracker = ProcessTracker::new(3);
        assert_eq!(tracker.add_process_record(proc.clone()).is_ok(), true);
        assert_eq!(tracker.add_process_record(proc.clone()).is_ok(), true);
        assert_eq!(tracker.add_process_record(proc.clone()).is_ok(), true);
        assert_eq!(tracker.procs.len(), 1);
        assert_eq!(tracker.procs[0].len(), 3);
    }

    #[test]
    fn process_records_cleaned() {
        let proc = Process::myself().unwrap();
        let mut tracker = ProcessTracker::new(3);
        assert_eq!(tracker.add_process_record(proc.clone()).is_ok(), true);
        assert_eq!(tracker.add_process_record(proc.clone()).is_ok(), true);
        assert_eq!(tracker.add_process_record(proc.clone()).is_ok(), true);
        assert_eq!(tracker.add_process_record(proc.clone()).is_ok(), true);
        assert_eq!(tracker.procs.len(), 1);
        assert_eq!(tracker.procs[0].len(), 3);
        assert_eq!(tracker.add_process_record(proc.clone()).is_ok(), true);
        assert_eq!(tracker.add_process_record(proc.clone()).is_ok(), true);
        assert_eq!(tracker.add_process_record(proc.clone()).is_ok(), true);
        assert_eq!(tracker.add_process_record(proc.clone()).is_ok(), true);
        assert_eq!(tracker.add_process_record(proc.clone()).is_ok(), true);
        assert_eq!(tracker.add_process_record(proc.clone()).is_ok(), true);
        assert_eq!(tracker.add_process_record(proc.clone()).is_ok(), true);
        assert_eq!(tracker.procs.len(), 1);
        assert_eq!(tracker.procs[0].len(), 3);
    }
}