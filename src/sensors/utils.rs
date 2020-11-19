use procfs::process::Process;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone)]
/// Manages ProcessRecord instances.
pub struct ProcessTracker {
    /// Each subvector keeps track of records for a given PID.
    pub procs: Vec<Vec<ProcessRecord>>,
    /// Maximum number of ProcessRecord instances that scaphandre is allowed to
    /// store, per PID (thus, for each subvector).
    pub max_records_per_process: u16
}

impl ProcessTracker {
    /// Instantiates ProcessTracker.
    /// 
    /// # Example:
    /// ```
    /// // 5 will be the maximum number of ProcessRecord instances
    /// // stored for each PID.
    /// let tracker = ProcessTracker::new(5);
    /// ``` 
    pub fn new(max_records_per_process: u16) -> ProcessTracker {
        ProcessTracker {
            procs: vec![],
            max_records_per_process
        }
    }

    /// Properly creates and adds a ProcessRecord to 'procs', the vector of vectors or ProcessRecords
    /// owned by the ProcessTracker instance. This method should be used to keep track of processes
    /// states during all the lifecycle of the exporter.
    /// # Example:
    /// ```
    /// use procfs::process::Process;
    /// let tracker = ProcessTracker::new(5);
    /// if let Ok(result) = tracker.add_process_record(
    ///     Process::new()
    /// ){
    ///     println!("ProcessRecord stored successfully: {}", result);
    /// }
    /// ```
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
            ProcessTracker::check_pid_changes(&process_record, vector);
            vector.insert(0, process_record); // we add the process record to the vector
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
                    b.timestamp.cmp(&a.timestamp)
                });
                debug!("Cleaning old ProcessRecords in vector for PID {}", records[0].process.pid);
                debug!("Deleting record with timestamp: {:?}", records.pop().unwrap().timestamp);
            }
        }
    }

    /// Ensure we don't store records for different processes in the same vector.
    fn check_pid_changes(record: &ProcessRecord, records: &mut Vec<ProcessRecord>) {
        if !records.is_empty() && record.process.stat.comm != records.get(0).unwrap().process.stat.comm {
            records.clear();
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

    /// Returns the result of the substraction of utime between last and
    /// previous ProcessRecord for a given pid.
    pub fn get_diff_utime(&self, pid: i32) -> Option<u64> {
        let records = self.find_records(pid).unwrap();
        if records.len() > 1 {
            return Some(records[0].process.stat.utime - records[1].process.stat.utime)
        }
        None
    }
    /// Returns the result of the substraction of stime between last and
    /// previous ProcessRecord for a given pid.
    pub fn get_diff_stime(&self, pid: i32) -> Option<u64> {
        let records = self.find_records(pid).unwrap();
        if records.len() > 1 {
            return Some(records[0].process.stat.stime - records[1].process.stat.stime)
        }
        None
    }

    /// Returns all vectors of process records linked to a running, sleeping, waiting or zombie process.
    /// (Not terminated)
    pub fn get_alive_processes(&self) -> Vec<&Vec<ProcessRecord>>{
        let mut res = vec![];
        for p in self.procs.iter() {
            if !p.is_empty() {
                let status = p[0].process.status();
                if status.is_ok() {
                    let status_val = status.unwrap();
                    if !&status_val.state.contains("T") { // !&status_val.state.contains("Z") && 
                        res.push(p);
                    }
                }
            }
        }
        res
    }

    /// Returns a vector containing pids of all running, sleeping or waiting current processes.
    pub fn get_alive_pids(&self) -> Vec<i32> {
        self.get_alive_processes().iter().filter(
            |x| !x.is_empty()
        ).map(
            |x| x[0].process.pid
        ).collect()
    }

    /// Returns a vector containing pids of all processes being tracked.
    pub fn get_all_pids(&self) -> Vec<i32> {
        self.procs.iter().filter(
            |x| !x.is_empty()
        ).map(
            |x| x[0].process.pid
        ).collect()
    }

    pub fn get_process_name(&self, pid: i32) -> String {
        let mut result = self.procs.iter().filter(|x| {
            !x.is_empty() && x.get(0).unwrap().process.pid == pid
        });
        let process = result.next().unwrap();
        if let Some(_) = result.next() {
            panic!("Found two vectors of processes with the same id, maintainers should fix this.");
        }
        process.get(0).unwrap().process.stat.comm.clone() 
    }

    pub fn get_process_cmdline(&self, pid: i32) -> Option<String> {
        let mut result = self.procs.iter().filter( |x| {
                !x.is_empty() && x.get(0).unwrap().process.pid == pid
            }
        );
        let process = result.next().unwrap();
        if let Some(vec) = process.get(0) {
            if let Ok(mut cmdline_vec) = vec.process.cmdline() {
                let mut cmdline = String::from("");
                while !cmdline_vec.is_empty() {
                    cmdline.push_str(&cmdline_vec.pop().unwrap());
                }
                return Some(cmdline);
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct ProcessRecord {
    pub process: Process,
    pub timestamp: Duration
}

impl ProcessRecord {
    pub fn new(process: Process) -> ProcessRecord {
        ProcessRecord {
            process,
            timestamp: current_system_time_since_epoch()
        }
    }

    pub fn total_time_jiffies(&self) -> u64 {
        let stime = self.process.stat.stime;
        let utime = self.process.stat.utime;
        let cutime = self.process.stat.cutime as u64;
        let cstime = self.process.stat.cstime as u64;
        let guest_time = self.process.stat.guest_time.unwrap_or_default();
        let cguest_time = self.process.stat.cguest_time.unwrap_or_default() as u64;
        let delayacct_blkio_ticks = self.process.stat.delayacct_blkio_ticks.unwrap_or_default();
        let itrealvalue = self.process.stat.itrealvalue as u64;

        debug!(
            "ProcessRecord: stime {} utime {} cutime {} cstime {} guest_time {} cguest_time {} delayacct_blkio_ticks {} itrealvalue {}",
            stime, utime, cutime, cstime, guest_time, cguest_time, delayacct_blkio_ticks, itrealvalue
        );

        // not including cstime and cutime in total as they are reported only when child dies
        // child metrics as already reported as the child processes are in the global process
        // list, found as /proc/PID/stat
        let total = stime + utime + guest_time + cguest_time + delayacct_blkio_ticks + itrealvalue;

        total
    }


}

pub fn current_system_time_since_epoch() -> Duration {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn process_records_added() {
        let proc = Process::myself().unwrap();
        let mut tracker = ProcessTracker::new(3);
        for _ in 0..2 {
            assert_eq!(tracker.add_process_record(proc.clone()).is_ok(), true);
        }
        assert_eq!(tracker.procs.len(), 1);
        assert_eq!(tracker.procs[0].len(), 3);
    }

    #[test]
    fn process_records_cleaned() {
        let proc = Process::myself().unwrap();
        let mut tracker = ProcessTracker::new(3);
        for _ in 0..5 {
            assert_eq!(tracker.add_process_record(proc.clone()).is_ok(), true);
        }
        assert_eq!(tracker.procs.len(), 1);
        assert_eq!(tracker.procs[0].len(), 3);
        for _ in 0..15 {
            assert_eq!(tracker.add_process_record(proc.clone()).is_ok(), true);
        }
        assert_eq!(tracker.procs.len(), 1);
        assert_eq!(tracker.procs[0].len(), 3);
    }

}