use ordered_float::*;
#[cfg(target_os = "linux")]
use procfs;
use regex::Regex;
#[allow(unused_imports)]
use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use sysinfo::{
    get_current_pid, CpuExt, CpuRefreshKind, Pid, Process, ProcessExt, ProcessStatus, System,
    SystemExt,
};
#[cfg(all(target_os = "linux", feature = "containers"))]
use {docker_sync::container::Container, k8s_sync::Pod};

pub struct IStatM {
    pub size: u64,
    pub resident: u64,
    pub shared: u64,
    pub text: u64,
    pub lib: u64,
    pub data: u64,
    pub dt: u64,
}

#[derive(Debug, Clone)]
pub struct IStat {
    pub pid: i32,
    pub comm: String,
    pub state: char,
    pub ppid: i32,
    pub pgrp: i32,
    pub session: i32,
    pub tty_nr: i32,
    pub tpgid: i32,
    pub flags: u32,
    pub utime: u64,
    pub stime: u64,
    pub cutime: i64,
    pub cstime: i64,
    pub nice: i64,
    pub num_threads: i64,
    pub itrealvalue: i64,
    pub starttime: u64,
    pub vsize: u64,
    pub signal: u64,
    pub blocked: u64,
    pub exit_signal: Option<i32>,
    pub processor: Option<i32>,
    pub delayacct_blkio_ticks: Option<u64>,
    pub guest_time: Option<u64>,
    pub cguest_time: Option<i64>,
    pub start_data: Option<u64>,
    pub end_data: Option<u64>,
    pub exit_code: Option<i32>,
}

#[derive(Clone)]
pub struct IStatus {
    pub name: String,
    pub umask: Option<u32>,
    pub state: String,
    pub pid: i32,
    pub ppid: i32,
}

#[derive(Debug, Clone)]
pub struct IProcess {
    pub pid: Pid,
    pub owner: u32,
    pub comm: String,
    pub cmdline: Vec<String>,
    //CPU (all of them) time usage, as a percentage
    pub cpu_usage_percentage: f32,
    // Virtual memory used by the process (at the time the struct is created), in bytes
    pub virtual_memory: u64,
    // Memory consumed by the process (at the time the struct is created), in bytes
    pub memory: u64,
    // Disk bytes read by the process
    pub disk_read: u64,
    // Disk bytes written by the process
    pub disk_written: u64,
    // Total disk bytes read by the process
    pub total_disk_read: u64,
    // Total disk bytes written by the process
    pub total_disk_written: u64,
    #[cfg(target_os = "linux")]
    pub stime: u64,
    #[cfg(target_os = "linux")]
    pub utime: u64,
}

impl IProcess {
    pub fn new(process: &Process) -> IProcess {
        let disk_usage = process.disk_usage();
        #[cfg(target_os = "linux")]
        {
            let mut stime = 0;
            let mut utime = 0;
            if let Ok(procfs_process) =
                procfs::process::Process::new(process.pid().to_string().parse::<i32>().unwrap())
            {
                if let Ok(stat) = procfs_process.stat() {
                    stime += stat.stime;
                    utime += stat.utime;
                }
            }
            IProcess {
                pid: process.pid(),
                owner: 0,
                comm: String::from(process.exe().to_str().unwrap()),
                cmdline: process.cmd().to_vec(),
                cpu_usage_percentage: process.cpu_usage(),
                memory: process.memory(),
                virtual_memory: process.virtual_memory(),
                disk_read: disk_usage.read_bytes,
                disk_written: disk_usage.written_bytes,
                total_disk_read: disk_usage.total_read_bytes,
                total_disk_written: disk_usage.total_written_bytes,
                stime,
                utime,
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            IProcess {
                pid: process.pid(),
                owner: 0,
                comm: String::from(process.exe().to_str().unwrap()),
                cmdline: process.cmd().to_vec(),
                cpu_usage_percentage: process.cpu_usage(),
                memory: process.memory(),
                virtual_memory: process.virtual_memory(),
                disk_read: disk_usage.read_bytes,
                disk_written: disk_usage.written_bytes,
                total_disk_read: disk_usage.total_read_bytes,
                total_disk_written: disk_usage.total_written_bytes,
            }
        }
    }

    /// Returns the command line of related to the process, as found by sysinfo.
    pub fn cmdline(&self, proc_tracker: &ProcessTracker) -> Result<Vec<String>, Error> {
        if let Some(p) = proc_tracker.sysinfo.process(self.pid) {
            Ok(p.cmd().to_vec())
        } else {
            Err(Error::new(
                ErrorKind::Other,
                "Failed to get original process.",
            ))
        }
    }

    /// Returns the executable string related to the process
    pub fn exe(&self, proc_tracker: &ProcessTracker) -> Result<PathBuf, String> {
        if let Some(p) = proc_tracker.sysinfo.process(self.pid) {
            Ok(PathBuf::from(p.exe().to_str().unwrap()))
        } else {
            Err(String::from("Couldn't get process."))
        }
    }

    #[cfg(target_os = "linux")]
    pub fn total_time_jiffies(&self, proc_tracker: &ProcessTracker) -> u64 {
        if let Some(rec) = proc_tracker.get_process_last_record(self.pid) {
            return rec.process.stime + rec.process.utime;
        }
        0
    }

    pub fn myself(proc_tracker: &ProcessTracker) -> Result<IProcess, String> {
        Ok(IProcess::new(
            proc_tracker
                .sysinfo
                .process(get_current_pid().unwrap())
                .unwrap(),
        ))
    }

    #[cfg(target_os = "linux")]
    pub fn cgroups() {}
}

pub fn page_size() -> Result<u64, String> {
    let res;
    #[cfg(target_os = "linux")]
    {
        res = Ok(procfs::page_size())
    }
    #[cfg(target_os = "windows")]
    {
        res = Ok(4096u64)
    }
    res
}

#[derive(Debug)]
/// Manages ProcessRecord instances.
pub struct ProcessTracker {
    /// Each subvector keeps track of records for a given PID.
    pub procs: Vec<Vec<ProcessRecord>>,
    /// Number of CPU cores to deal with
    pub nb_cores: usize,
    /// Maximum number of ProcessRecord instances that scaphandre is allowed to
    /// store, per PID (thus, for each subvector).
    pub max_records_per_process: u16,
    /// Sysinfo system for resources monitoring
    pub sysinfo: System,
    #[cfg(feature = "containers")]
    pub regex_cgroup_docker: Regex,
    #[cfg(feature = "containers")]
    pub regex_cgroup_kubernetes: Regex,
    #[cfg(feature = "containers")]
    pub regex_cgroup_containerd: Regex,
}

impl Clone for ProcessTracker {
    fn clone(&self) -> ProcessTracker {
        ProcessTracker {
            procs: self.procs.clone(),
            max_records_per_process: self.max_records_per_process,
            sysinfo: System::new_all(),
            #[cfg(feature = "containers")]
            regex_cgroup_docker: self.regex_cgroup_docker.clone(),
            #[cfg(feature = "containers")]
            regex_cgroup_kubernetes: self.regex_cgroup_kubernetes.clone(),
            #[cfg(feature = "containers")]
            regex_cgroup_containerd: self.regex_cgroup_containerd.clone(),
            nb_cores: self.nb_cores,
        }
    }
}

impl ProcessTracker {
    /// Instantiates ProcessTracker.
    ///
    /// # Example:
    /// ```
    /// // 5 will be the maximum number of ProcessRecord instances
    /// // stored for each PID.
    /// use scaphandre::sensors::utils::ProcessTracker;
    /// let tracker = ProcessTracker::new(5);
    /// ```
    pub fn new(max_records_per_process: u16) -> ProcessTracker {
        #[cfg(feature = "containers")]
        let regex_cgroup_docker = Regex::new(r"^.*/docker.*$").unwrap();
        #[cfg(feature = "containers")]
        let regex_cgroup_kubernetes = Regex::new(r"^/kubepods.*$").unwrap();
        #[cfg(feature = "containers")]
        let regex_cgroup_containerd = Regex::new("/system.slice/containerd.service/.*$").unwrap();

        let mut system = System::new_all();
        system.refresh_cpu_specifics(CpuRefreshKind::everything());
        let nb_cores = system.cpus().len();

        ProcessTracker {
            procs: vec![],
            max_records_per_process,
            sysinfo: system,
            #[cfg(feature = "containers")]
            regex_cgroup_docker,
            #[cfg(feature = "containers")]
            regex_cgroup_kubernetes,
            #[cfg(feature = "containers")]
            regex_cgroup_containerd,
            nb_cores,
        }
    }

    pub fn refresh(&mut self) {
        self.sysinfo.refresh_components();
        self.sysinfo.refresh_memory();
        self.sysinfo.refresh_disks();
        self.sysinfo.refresh_disks_list();
        self.sysinfo
            .refresh_cpu_specifics(CpuRefreshKind::everything());
    }

    pub fn components(&mut self) -> Vec<String> {
        let mut res = vec![];
        for c in self.sysinfo.components() {
            res.push(format!("{c:?}"));
        }
        res
    }

    /// Properly creates and adds a ProcessRecord to 'procs', the vector of vectors or ProcessRecords
    /// owned by the ProcessTracker instance. This method should be used to keep track of processes
    /// states during all the lifecycle of the exporter.
    /// # Linux Example:
    /// ```
    /// use scaphandre::sensors::utils::{ProcessTracker, IProcess};
    /// use scaphandre::sensors::Topology;
    /// use std::collections::HashMap;
    /// use sysinfo::SystemExt;
    /// let mut pt = ProcessTracker::new(5);
    /// pt.sysinfo.refresh_processes();
    /// pt.sysinfo.refresh_cpu();
    /// let current_procs = pt
    ///     .sysinfo
    ///     .processes()
    ///     .values()
    ///     .map(IProcess::new)
    ///     .collect::<Vec<_>>();
    /// for p in current_procs {
    ///     match pt.add_process_record(p) {
    ///         Ok(result) => { println!("ProcessRecord stored successfully: {}", result); }
    ///         Err(msg) => {
    ///             panic!("Failed to track process !\nGot: {}", msg)
    ///         }
    ///     }
    /// }
    /// ```
    pub fn add_process_record(&mut self, process: IProcess) -> Result<String, String> {
        let iterator = self.procs.iter_mut();
        let pid = process.pid;
        // find the vector containing Process instances with the same pid
        let mut filtered = iterator.filter(|x| !x.is_empty() && x[0].process.pid == pid);
        let result = filtered.next();
        let process_record = ProcessRecord::new(process);
        if let Some(vector) = result {
            // if a vector of process records has been found
            // check if the previous records in the vector are from the same process
            // (if the process with that pid is not a new one) and if so, drop it for a new one
            if !vector.is_empty()
                && process_record.process.comm != vector.first().unwrap().process.comm
            {
                *vector = vec![];
            }
            //ProcessTracker::check_pid_changes(&process_record, vector);
            vector.insert(0, process_record); // we add the process record to the vector
            ProcessTracker::clean_old_process_records(vector, self.max_records_per_process);
        } else {
            // if no vector of process records with the same pid has been found in self.procs
            self.procs.push(vec![process_record]); // we create a new vector in self.procs
        }

        Ok(String::from("Successfully added record to process."))
    }

    pub fn get_process_last_record(&self, pid: Pid) -> Option<&ProcessRecord> {
        if let Some(records) = self.find_records(pid) {
            if let Some(last) = records.first() {
                return Some(last);
            }
        }
        None
    }

    /// Removes as many ProcessRecords as needed from the vector (passed as a mutable ref in parameters)
    /// in order for the vector length to match self.max_records_per_process.
    fn clean_old_process_records(records: &mut Vec<ProcessRecord>, max_records_per_process: u16) {
        if records.len() > max_records_per_process as usize {
            let diff = records.len() - max_records_per_process as usize;
            for _ in 0..diff {
                records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                let res = records.pop().unwrap().timestamp;
                trace!(
                    "Cleaning old ProcessRecords in vector for PID {}",
                    records[0].process.pid
                );
                trace!("Deleting record with timestamp: {:?}", res);
            }
        }
    }

    /// Returns a Some(ref to vector of ProcessRecords) if the pid is found
    /// in self.procs. Returns None otherwise.
    pub fn find_records(&self, pid: Pid) -> Option<&Vec<ProcessRecord>> {
        let mut refer = None;
        for v in &self.procs {
            if !v.is_empty() && v[0].process.pid == pid {
                if refer.is_some() {
                    warn!("ISSUE: PID {} spread in proc tracker", pid);
                }
                refer = Some(v);
            }
        }
        refer
    }

    pub fn get_cpu_frequency(&self) -> u64 {
        self.sysinfo.global_cpu_info().frequency()
    }

    /// Returns all vectors of process records linked to a running, sleeping, waiting or zombie process.
    /// (Not terminated)
    pub fn get_alive_processes(&self) -> Vec<&Vec<ProcessRecord>> {
        trace!("In get alive processes.");
        let mut res = vec![];
        for p in self.procs.iter() {
            //#[cfg(target_os = "linux")]
            //if !p.is_empty() {
            //    let status = p[0].process.status();
            //    if let Ok(status_val) = status {
            //        if !&status_val.state.contains('T') {
            //            // !&status_val.state.contains("Z") &&
            //            res.push(p);
            //        }
            //    }
            //}
            if !p.is_empty() {
                //TODO implement
                // clippy will ask you to remove mut from res, but you just need to implement to fix that
                if let Some(sysinfo_p) = self.sysinfo.process(p[0].process.pid) {
                    let status = sysinfo_p.status();
                    if status != ProcessStatus::Dead {
                        //&& status != ProcessStatus::Stop {
                        res.push(p);
                    }
                }
            }
        }
        trace!("End of get alive processes.");
        res
    }

    /// Extracts the container_id from a cgroup path containing it.
    #[cfg(feature = "containers")]
    fn extract_pod_id_from_cgroup_path(&self, pathname: String) -> Result<String, std::io::Error> {
        let mut container_id = String::from(pathname.split('/').last().unwrap());
        if container_id.starts_with("docker-") {
            container_id = container_id.strip_prefix("docker-").unwrap().to_string();
        }
        if container_id.ends_with(".scope") {
            container_id = container_id.strip_suffix(".scope").unwrap().to_string();
        }
        if container_id.contains("cri-containerd") {
            container_id = container_id.split(':').last().unwrap().to_string();
        }
        Ok(container_id)
    }

    /// Returns a HashMap containing labels (key + value) to be attached to
    /// the metrics of the process referenced by its pid.
    /// The *containers* slice contains the [Container] items referencing
    /// currently running docker containers on the machine.
    /// The *pods* slice contains the [Pod] items referencing currently
    /// running pods on the machine if it is a kubernetes cluster node.
    #[cfg(feature = "containers")]
    pub fn get_process_container_description(
        &self,
        pid: Pid, // the PID of the process to look for
        containers: &[Container],
        docker_version: String,
        pods: &[Pod],
        //kubernetes_version: String,
    ) -> HashMap<String, String> {
        let mut result = self.procs.iter().filter(
            // get all processes that have process records
            |x| !x.is_empty() && x.first().unwrap().process.pid == pid,
        );
        let process = result.next().unwrap();
        let mut description = HashMap::new();
        let regex_clean_container_id = Regex::new("[[:alnum:]]{12,}").unwrap();
        if let Some(_p) = process.first() {
            // if we have the cgroups data from the original process struct
            if let Ok(procfs_process) =
                procfs::process::Process::new(pid.to_string().parse::<i32>().unwrap())
            {
                if let Ok(cgroups) = procfs_process.cgroups() {
                    let mut found = false;
                    for cg in &cgroups {
                        if found {
                            break;
                        }
                        // docker
                        if self.regex_cgroup_docker.is_match(&cg.pathname) {
                            debug!("regex docker matched : {}", &cg.pathname); //coucou
                            description.insert(
                                String::from("container_scheduler"),
                                String::from("docker"),
                            );
                            // extract container_id
                            //let container_id = cg.pathname.split('/').last().unwrap();
                            if let Some(container_id_capture) =
                                regex_clean_container_id.captures(&cg.pathname)
                            {
                                let container_id = &container_id_capture[0];
                                debug!("container_id = {}", container_id);
                                description.insert(
                                    String::from("container_id"),
                                    String::from(container_id),
                                );
                                if let Some(container) =
                                    containers.iter().find(|x| x.Id == container_id)
                                {
                                    debug!("found container with id: {}", &container_id);
                                    let mut names = String::from("");
                                    for n in &container.Names {
                                        debug!(
                                            "adding container name: {}",
                                            &n.trim().replace('/', "")
                                        );
                                        names.push_str(&n.trim().replace('/', ""));
                                    }
                                    description.insert(String::from("container_names"), names);
                                    description.insert(
                                        String::from("container_docker_version"),
                                        docker_version.clone(),
                                    );
                                    if let Some(labels) = &container.Labels {
                                        for (k, v) in labels {
                                            let escape_list = ["-", ".", ":", " "];
                                            let mut key = k.clone();
                                            for e in escape_list.iter() {
                                                key = key.replace(e, "_");
                                            }
                                            description.insert(
                                                format!("container_label_{key}"),
                                                v.to_string(),
                                            );
                                        }
                                    }
                                }
                                found = true;
                            }
                        } else {
                            // containerd
                            if self.regex_cgroup_containerd.is_match(&cg.pathname) {
                                debug!("regex containerd matched : {}", &cg.pathname);
                                description.insert(
                                    String::from("container_runtime"),
                                    String::from("containerd"),
                                );
                            } else if self.regex_cgroup_kubernetes.is_match(&cg.pathname) {
                                debug!("regex kubernetes matched : {}", &cg.pathname);
                                // kubernetes not using containerd but we can get the container id
                            } else {
                                // cgroup not related to a container technology
                                continue;
                            }

                            let container_id =
                                match self.extract_pod_id_from_cgroup_path(cg.pathname.clone()) {
                                    Ok(id) => id,
                                    Err(err) => {
                                        info!("Couldn't get container id : {}", err);
                                        "ERROR Couldn't get container id".to_string()
                                    }
                                };
                            description.insert(String::from("container_id"), container_id.clone());
                            // find pod in pods that has pod_status > container_status.container
                            if let Some(pod) = pods.iter().find(|x| match &x.status {
                                Some(status) => {
                                    if let Some(container_statuses) = &status.container_statuses {
                                        container_statuses.iter().any(|y| match &y.container_id {
                                            Some(id) => {
                                                if let Some(final_id) = id.strip_prefix("docker://")
                                                {
                                                    final_id == container_id
                                                } else if let Some(final_id) =
                                                    id.strip_prefix("containerd://")
                                                {
                                                    final_id == container_id
                                                } else {
                                                    false
                                                }
                                            }
                                            None => false,
                                        })
                                    } else {
                                        false
                                    }
                                }
                                None => false,
                            }) {
                                description.insert(
                                    String::from("container_scheduler"),
                                    String::from("kubernetes"),
                                );
                                if let Some(pod_name) = &pod.metadata.name {
                                    description.insert(
                                        String::from("kubernetes_pod_name"),
                                        pod_name.clone(),
                                    );
                                }
                                if let Some(pod_namespace) = &pod.metadata.namespace {
                                    description.insert(
                                        String::from("kubernetes_pod_namespace"),
                                        pod_namespace.clone(),
                                    );
                                }
                                if let Some(pod_spec) = &pod.spec {
                                    if let Some(node_name) = &pod_spec.node_name {
                                        description.insert(
                                            String::from("kubernetes_node_name"),
                                            node_name.clone(),
                                        );
                                    }
                                }
                            }
                            found = true;
                        } //else {
                          //    debug!("Cgroup not identified as related to a container technology : {}", &cg.pathname);
                          //}
                    }
                }
            } else {
                debug!("Could'nt find {} in procfs.", pid.to_string());
            }
        }
        description
    }

    /// Returns a vector containing pids of all running, sleeping or waiting current processes.
    pub fn get_alive_pids(&self) -> Vec<Pid> {
        self.get_alive_processes()
            .iter()
            .filter(|x| !x.is_empty())
            .map(|x| x[0].process.pid)
            .collect()
    }

    /// Returns a vector containing pids of all processes being tracked.
    pub fn get_all_pids(&self) -> Vec<Pid> {
        self.procs
            .iter()
            .filter(|x| !x.is_empty())
            .map(|x| x[0].process.pid)
            .collect()
    }

    /// Returns the process name associated to a PID
    pub fn get_process_name(&self, pid: Pid) -> String {
        let mut result = self
            .procs
            .iter()
            .filter(|x| !x.is_empty() && x.first().unwrap().process.pid == pid);
        let process = result.next().unwrap();
        if result.next().is_some() {
            panic!("Found two vectors of processes with the same id, maintainers should fix this.");
        }

        debug!("End of get process name.");
        process.first().unwrap().process.comm.clone()
    }

    /// Returns the cmdline string associated to a PID
    pub fn get_process_cmdline(&self, pid: Pid) -> Option<String> {
        let mut result = self
            .procs
            .iter()
            .filter(|x| !x.is_empty() && x.first().unwrap().process.pid == pid);
        let process = result.next().unwrap();
        if let Some(p) = process.first() {
            let cmdline_request = p.process.cmdline(self);
            if let Ok(mut cmdline_vec) = cmdline_request {
                let mut cmdline = String::from("");
                while !cmdline_vec.is_empty() {
                    if !cmdline_vec.is_empty() {
                        cmdline.push_str(&cmdline_vec.remove(0));
                    }
                }
                return Some(cmdline);
            }
        }
        None
    }

    pub fn get_cpu_usage_percentage(&self, pid: Pid, nb_cores: usize) -> f32 {
        let cpu_current_usage = self.sysinfo.global_cpu_info().cpu_usage();
        if let Some(p) = self.sysinfo.process(pid) {
            (cpu_current_usage * p.cpu_usage() / 100.0) / nb_cores as f32
        } else {
            0.0
        }
    }

    /// Returns processes sorted by the highest consumers in first
    pub fn get_top_consumers(&self, top: u16) -> Vec<(IProcess, f64)> {
        let mut consumers: Vec<(IProcess, OrderedFloat<f64>)> = vec![];
        for p in &self.procs {
            if p.len() > 1 {
                let diff = self
                    .get_cpu_usage_percentage(p.first().unwrap().process.pid as _, self.nb_cores);
                if consumers
                    .iter()
                    .filter(|x| {
                        if let Some(p) = self.sysinfo.process(x.0.pid as _) {
                            return p.cpu_usage() > diff;
                        }
                        false
                    })
                    .count()
                    < top as usize
                {
                    let pid = p.first().unwrap().process.pid;
                    if let Some(sysinfo_process) = self.sysinfo.process(pid as _) {
                        let new_consumer = IProcess::new(sysinfo_process);
                        consumers.push((new_consumer, OrderedFloat(diff as f64)));
                        consumers.sort_by(|x, y| y.1.cmp(&x.1));
                        if consumers.len() > top as usize {
                            consumers.pop();
                        }
                    } else {
                        debug!("Couldn't get process info for {}", pid);
                    }
                }
            }
        }
        let mut result: Vec<(IProcess, f64)> = vec![];
        for (p, f) in consumers {
            result.push((p, f.into_inner()));
        }
        result
    }

    /// Returns processes filtered by a regexp
    pub fn get_filtered_processes(&self, regex_filter: &Regex) -> Vec<(IProcess, f64)> {
        let mut consumers: Vec<(IProcess, OrderedFloat<f64>)> = vec![];
        for p in &self.procs {
            if p.len() > 1 {
                let diff = self
                    .get_cpu_usage_percentage(p.first().unwrap().process.pid as _, self.nb_cores);
                let p_record = p.last().unwrap();
                let process_exe = p_record.process.exe(self).unwrap_or_default();
                let process_cmdline = p_record.process.cmdline(self).unwrap_or_default();
                if regex_filter.is_match(process_exe.to_str().unwrap_or_default()) {
                    consumers.push((p_record.process.clone(), OrderedFloat(diff as f64)));
                    consumers.sort_by(|x, y| y.1.cmp(&x.1));
                } else if regex_filter.is_match(&process_cmdline.concat()) {
                    consumers.push((p_record.process.clone(), OrderedFloat(diff as f64)));
                    consumers.sort_by(|x, y| y.1.cmp(&x.1));
                }
            }
        }
        let mut result: Vec<(IProcess, f64)> = vec![];
        for (p, f) in consumers {
            result.push((p, f.into_inner()));
        }
        result
    }

    /// Drops a vector of ProcessRecord instances from self.procs
    /// if the last ProcessRecord from the vector is of state Terminated
    /// (if the process is not running anymore)
    pub fn clean_terminated_process_records_vectors(&mut self) {
        //TODO get stats from processes to know what is hapening !
        for v in &mut self.procs {
            if !v.is_empty() {
                if let Some(first) = v.first() {
                    if let Some(p) = self.sysinfo.process(first.process.pid) {
                        match p.status() {
                            ProcessStatus::Idle => {}
                            ProcessStatus::Dead => {}
                            ProcessStatus::Stop => {
                                while !v.is_empty() {
                                    v.pop();
                                }
                            }
                            ProcessStatus::Run => {}
                            ProcessStatus::LockBlocked => {}
                            ProcessStatus::Waking => {}
                            ProcessStatus::Wakekill => {}
                            ProcessStatus::Tracing => {}
                            ProcessStatus::Zombie => {}
                            ProcessStatus::Sleep => {}
                            ProcessStatus::Parked => {}
                            ProcessStatus::UninterruptibleDiskSleep => {}
                            ProcessStatus::Unknown(_code) => {}
                        }
                    } else {
                        while !v.is_empty() {
                            v.pop();
                        }
                    }
                }
            }
        }
        self.drop_empty_process_records_vectors();
    }

    /// Removes empty Vectors from self.procs
    fn drop_empty_process_records_vectors(&mut self) {
        let procs = &mut self.procs;
        if !procs.is_empty() {
            for i in 0..(procs.len() - 1) {
                if let Some(v) = procs.get(i) {
                    if v.is_empty() {
                        procs.remove(i);
                    }
                }
            }
        }
    }
}

/// Stores the information of a give process at a given timestamp
#[derive(Debug, Clone)]
pub struct ProcessRecord {
    //TODO: abstract from Process procfs
    pub process: IProcess,
    pub timestamp: Duration,
}

impl ProcessRecord {
    /// Instanciates ProcessRecord and returns the instance, with timestamp set to the current
    /// system time since epoch
    pub fn new(process: IProcess) -> ProcessRecord {
        ProcessRecord {
            process,
            timestamp: current_system_time_since_epoch(),
        }
    }
}

/// Returns a Duration instance with the current timestamp
pub fn current_system_time_since_epoch() -> Duration {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
}

mod tests {

    #[test]
    fn process_cmdline() {
        use super::*;
        use crate::sensors::Topology;
        // find the cmdline of current proc thanks to sysinfo
        // do the same with processtracker
        // assert
        let mut system = System::new();
        system.refresh_all();
        let self_pid_by_sysinfo = get_current_pid();
        let self_process_by_sysinfo = system.process(self_pid_by_sysinfo.unwrap()).unwrap();

        let mut topo = Topology::new(HashMap::new());
        topo.refresh();
        let self_process_by_scaph = IProcess::myself(&topo.proc_tracker).unwrap();

        assert_eq!(
            self_process_by_sysinfo.cmd().concat(),
            topo.proc_tracker
                .get_process_cmdline(self_process_by_scaph.pid)
                .unwrap()
        );
    }

    #[cfg(all(test, target_os = "linux"))]
    #[test]
    fn process_records_added() {
        use super::*;
        use crate::sensors::Topology;
        let mut topo = Topology::new(HashMap::new());
        topo.refresh();
        let proc = IProcess::myself(&topo.proc_tracker).unwrap();
        let mut tracker = ProcessTracker::new(3);
        for _ in 0..3 {
            assert_eq!(tracker.add_process_record(proc.clone()).is_ok(), true);
        }
        assert_eq!(tracker.procs.len(), 1);
        assert_eq!(tracker.procs[0].len(), 3);
    }

    #[cfg(all(test, target_os = "linux"))]
    #[test]
    fn process_records_cleaned() {
        use super::*;
        let mut tracker = ProcessTracker::new(3);
        let proc = IProcess::myself(&tracker).unwrap();
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

//  Copyright 2020 The scaphandre authors.
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
