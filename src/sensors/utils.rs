#[cfg(target_os = "linux")]
use procfs::{self, process::Process};
use regex::Regex;
#[cfg(feature = "containers")]
use std::collections::HashMap;
#[cfg(target_os = "windows")]
use sysinfo::{get_current_pid, Process, ProcessExt, ProcessorExt, System, SystemExt};
//use std::error::Error;
use ordered_float::*;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
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
    //pub minflt: u64,
    //pub cminflt: u64,
    //pub majflt: u64,
    //pub cmajflt: u64,
    pub utime: u64,
    pub stime: u64,
    pub cutime: i64,
    pub cstime: i64,
    //pub priority: i64,
    pub nice: i64,
    pub num_threads: i64,
    pub itrealvalue: i64,
    pub starttime: u64,
    pub vsize: u64,
    //pub rss: i64,
    //pub rsslim: u64,
    //pub startcode: u64,
    //pub endcode: u64,
    //pub startstack: u64,
    //pub kstkesp: u64,
    //pub kstkeip: u64,
    pub signal: u64,
    pub blocked: u64,
    //pub sigignore: u64,
    //pub sigcatch: u64,
    //pub wchan: u64,
    //pub nswap: u64,
    //pub cnswap: u64,
    pub exit_signal: Option<i32>,
    pub processor: Option<i32>,
    //pub rt_priority: Option<u32>,
    //pub policy: Option<u32>,
    pub delayacct_blkio_ticks: Option<u64>,
    pub guest_time: Option<u64>,
    pub cguest_time: Option<i64>,
    pub start_data: Option<u64>,
    pub end_data: Option<u64>,
    //pub start_brk: Option<u64>,
    //pub arg_start: Option<u64>,
    //pub arg_end: Option<u64>,
    //pub env_start: Option<u64>,
    //pub env_end: Option<u64>,
    pub exit_code: Option<i32>,
}

impl IStat {
    #[cfg(target_os = "linux")]
    fn from_procfs_stat(stat: &procfs::process::Stat) -> IStat {
        IStat {
            blocked: stat.blocked,
            cguest_time: stat.cguest_time,
            comm: stat.comm.clone(),
            cstime: stat.cstime,
            cutime: stat.cutime,
            delayacct_blkio_ticks: stat.delayacct_blkio_ticks,
            end_data: stat.end_data,
            exit_code: stat.exit_code,
            exit_signal: stat.exit_signal,
            flags: stat.flags,
            guest_time: stat.guest_time,
            itrealvalue: stat.itrealvalue,
            nice: stat.nice,
            num_threads: stat.num_threads,
            pgrp: stat.pgrp,
            pid: stat.pid,
            ppid: stat.ppid,
            processor: stat.processor,
            session: stat.session,
            signal: stat.signal,
            start_data: stat.start_data,
            starttime: stat.starttime,
            state: stat.state,
            stime: stat.stime,
            tpgid: stat.tpgid,
            tty_nr: stat.tty_nr,
            utime: stat.utime,
            vsize: stat.vsize,
        }
    }

    #[cfg(target_os = "windows")]
    fn from_windows_process_stat(_process: &Process) -> IStat {
        IStat {
            blocked: 0,
            cguest_time: Some(0),
            comm: String::from("Not implemented yet !"),
            cstime: 0,
            cutime: 0,
            delayacct_blkio_ticks: Some(0),
            end_data: Some(0),
            exit_code: Some(0),
            exit_signal: Some(0),
            flags: 0,
            guest_time: Some(0),
            itrealvalue: 0,
            nice: 0,
            num_threads: 0,
            pgrp: 0,
            pid: 0,
            ppid: 0,
            processor: Some(0),
            session: 0,
            signal: 0,
            start_data: Some(0),
            starttime: 0,
            state: 'X',
            stime: 0,
            tpgid: 0,
            tty_nr: 0,
            utime: 0,
            vsize: 0,
        }
    }
}

#[derive(Clone)]
pub struct IStatus {
    pub name: String,
    pub umask: Option<u32>,
    pub state: String,
    //pub tgid: i32,
    //pub ngid: Option<i32>,
    pub pid: i32,
    pub ppid: i32,
    //pub tracerpid: i32,
    //pub ruid: u32,
    //pub euid: u32,
    //pub suid: u32,
    //pub fuid: u32,
    //pub rgid: u32,
    //pub egid: u32,
    //pub sgid: u32,
    //pub fgid: u32,
    //pub fdsize: u32,
    //pub groups: Vec<i32>,
    //pub nstgid: Option<Vec<i32>>,
    //pub nspid: Option<Vec<i32>>,
    //pub nspgid: Option<Vec<i32>>,
    //pub nssid: Option<Vec<i32>>,
    //pub vmpeak: Option<u64>,
    //pub vmsize: Option<u64>,
    //pub vmlck: Option<u64>,
    //pub vmpin: Option<u64>,
    //pub vmhwm: Option<u64>,
    //pub vmrss: Option<u64>,
    //pub rssanon: Option<u64>,
    //pub rssfile: Option<u64>,
    //pub rssshmem: Option<u64>,
    //pub vmdata: Option<u64>,
    //pub vmstk: Option<u64>,
    //pub vmexe: Option<u64>,
    //pub vmlib: Option<u64>,
    //pub vmpte: Option<u64>,
    //pub vmswap: Option<u64>,
    //pub hugetlbpages: Option<u64>,
    //pub threads: u64,
    //pub sigq: (u64, u64),
    //pub sigpnd: u64,
    //pub shdpnd: u64,
    //pub sigblk: u64,
    //pub sigign: u64,
    //pub sigcgt: u64,
    //pub capinh: u64,
    //pub capprm: u64,
    //pub capeff: u64,
    //pub capbnd: Option<u64>,
    //pub capamb: Option<u64>,
    //pub nonewprivs: Option<u64>,
    //pub seccomp: Option<u32>,
    //pub speculation_store_bypass: Option<String>,
    //pub cpus_allowed: Option<Vec<u32>>,
    //pub cpus_allowed_list: Option<Vec<(u32, u32)>>,
    //pub mems_allowed: Option<Vec<u32>>,
    //pub mems_allowed_list: Option<Vec<(u32, u32)>>,
    //pub voluntary_ctxt_switches: Option<u64>,
    //pub nonvoluntary_ctxt_switches: Option<u64>,
    //pub core_dumping: Option<bool>,
    //pub thp_enabled: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct IProcess {
    pub pid: i32,
    pub owner: u32,
    pub comm: String,
    pub cmdline: Vec<String>,
    pub stat: Option<IStat>,
    //pub root: Option<String>,
    #[cfg(target_os = "linux")]
    pub original: Process,
}

impl IProcess {
    #[cfg(target_os = "linux")]
    pub fn from_linux_process(process: &Process) -> IProcess {
        //let root = process.root();
        let mut cmdline = vec![String::from("")];
        if let Ok(raw_cmdline) = process.cmdline() {
            cmdline = raw_cmdline;
        }
        IProcess {
            pid: process.pid,
            owner: process.owner,
            original: process.clone(),
            comm: process.stat.comm.clone(),
            cmdline,
            stat: Some(IStat::from_procfs_stat(&process.stat)),
        }
    }

    #[cfg(target_os = "windows")]
    pub fn from_windows_process(process: &Process) -> IProcess {
        IProcess {
            pid: process.pid() as i32,
            owner: 0,
            comm: String::from(process.exe().to_str().unwrap()),
            cmdline: process.cmd().to_vec(),
            stat: Some(IStat::from_windows_process_stat(process)),
        }
    }

    #[cfg(target_os = "linux")]
    pub fn cmdline(&self) -> Result<Vec<String>, String> {
        if let Ok(cmdline) = self.original.cmdline() {
            Ok(cmdline)
        } else {
            Err(String::from("cmdline() was none"))
        }
    }
    #[cfg(target_os = "windows")]
    pub fn cmdline(&self, proc_tracker: &ProcessTracker) -> Result<Vec<String>, String> {
        if let Some(p) = proc_tracker.sysinfo.process(self.pid as usize) {
            Ok(p.cmd().to_vec())
        } else {
            Err(String::from("Failed to get original process."))
        }
    }

    pub fn statm(&self) -> Result<IStatM, String> {
        #[cfg(target_os = "linux")]
        {
            let mystatm = self.original.statm().unwrap();
            Ok(IStatM {
                size: mystatm.size,
                data: mystatm.data,
                dt: mystatm.dt,
                lib: mystatm.lib,
                resident: mystatm.resident,
                shared: mystatm.shared,
                text: mystatm.text,
            })
        }
        #[cfg(target_os = "windows")]
        Ok(IStatM {
            size: 42,
            data: 42,
            dt: 42,
            lib: 42,
            resident: 42,
            shared: 42,
            text: 42,
        })
    }

    #[cfg(target_os = "linux")]
    pub fn exe(&self) -> Result<PathBuf, String> {
        let original_exe = self.original.exe().unwrap();
        Ok(original_exe)
    }
    #[cfg(target_os = "windows")]
    pub fn exe(&self, proc_tracker: &ProcessTracker) -> Result<PathBuf, String> {
        if let Some(p) = proc_tracker.sysinfo.process(self.pid as usize) {
            Ok(PathBuf::from(p.exe().to_str().unwrap()))
        } else {
            Err(String::from("Couldn't get process."))
        }
    }

    pub fn status(&self) -> Result<IStatus, String> {
        #[cfg(target_os = "linux")]
        {
            if let Ok(original_status) = self.original.status() {
                let status = IStatus {
                    name: original_status.name,
                    pid: original_status.pid,
                    ppid: original_status.ppid,
                    state: original_status.state,
                    umask: original_status.umask,
                };
                Ok(status)
            } else {
                Err(format!("Couldn't get status for {}", self.pid))
            }
        }
        #[cfg(target_os = "windows")]
        {
            Ok(IStatus {
                name: String::from("Not implemented yet !"),
                pid: 42,
                ppid: 42,
                state: String::from("X"),
                umask: None,
            })
        }
    }

    #[cfg(target_os = "linux")]
    pub fn myself() -> Result<IProcess, String> {
        Ok(IProcess::from_linux_process(&Process::myself().unwrap()))
    }
    #[cfg(target_os = "windows")]
    pub fn myself(proc_tracker: &ProcessTracker) -> Result<IProcess, String> {
        Ok(IProcess::from_windows_process(
            proc_tracker
                .sysinfo
                .process(get_current_pid().unwrap() as usize)
                .unwrap(),
        ))
    }
}

pub fn page_size() -> Result<i64, String> {
    let res;
    #[cfg(target_os = "linux")]
    {
        res = Ok(procfs::page_size().unwrap())
    }
    #[cfg(target_os = "windows")]
    {
        res = Ok(4096)
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
    #[cfg(target_os = "windows")]
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
            #[cfg(target_os = "windows")]
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

        ProcessTracker {
            procs: vec![],
            max_records_per_process,
            #[cfg(target_os = "windows")]
            sysinfo: System::new_all(),
            #[cfg(feature = "containers")]
            regex_cgroup_docker,
            #[cfg(feature = "containers")]
            regex_cgroup_kubernetes,
            #[cfg(feature = "containers")]
            regex_cgroup_containerd,
            #[cfg(target_os = "windows")]
            nb_cores: System::new_all().processors().len(),
            #[cfg(target_os = "linux")]
            nb_cores: 0, // TODO implement
        }
    }

    /// Properly creates and adds a ProcessRecord to 'procs', the vector of vectors or ProcessRecords
    /// owned by the ProcessTracker instance. This method should be used to keep track of processes
    /// states during all the lifecycle of the exporter.
    /// # Linux Example:
    /// ```
    /// use procfs::process::Process;
    /// use scaphandre::sensors::utils::{ProcessTracker, IProcess};
    /// let mut tracker = ProcessTracker::new(5);
    /// let pid = 1;
    /// if let Ok(result) = tracker.add_process_record(
    ///     IProcess::from_linux_process(&Process::new(pid).unwrap())
    /// ){
    ///     println!("ProcessRecord stored successfully: {}", result);
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
                && process_record.process.comm != vector.get(0).unwrap().process.comm
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
    pub fn find_records(&self, pid: i32) -> Option<&Vec<ProcessRecord>> {
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

    /// Returns the result of the substraction of utime between last and
    /// previous ProcessRecord for a given pid.
    pub fn get_diff_utime(&self, pid: i32) -> Option<u64> {
        let records = self.find_records(pid).unwrap();
        if records.len() > 1 {
            if let Some(previous) = &records[0].process.stat {
                if let Some(current) = &records[1].process.stat {
                    return Some(previous.utime - current.utime);
                }
            }
        }
        None
    }
    /// Returns the result of the substraction of stime between last and
    /// previous ProcessRecord for a given pid.
    pub fn get_diff_stime(&self, pid: i32) -> Option<u64> {
        let records = self.find_records(pid).unwrap();
        if records.len() > 1 {
            if let Some(previous) = &records[0].process.stat {
                if let Some(current) = &records[1].process.stat {
                    return Some(previous.stime - current.stime);
                }
            }
        }
        None
    }

    /// Returns all vectors of process records linked to a running, sleeping, waiting or zombie process.
    /// (Not terminated)
    pub fn get_alive_processes(&self) -> Vec<&Vec<ProcessRecord>> {
        debug!("In get alive processes.");
        let mut res = vec![];
        for p in self.procs.iter() {
            #[cfg(target_os = "linux")]
            if !p.is_empty() {
                let status = p[0].process.status();
                if let Ok(status_val) = status {
                    if !&status_val.state.contains('T') {
                        // !&status_val.state.contains("Z") &&
                        res.push(p);
                    }
                }
            }
            #[cfg(target_os = "windows")]
            if !p.is_empty() {
                //TODO implement
                // clippy will ask you to remove mut from res, but you just need to implement to fix that
                if let Some(_sysinfo_p) = self.sysinfo.process(p[0].process.pid as usize) {
                    //let status = sysinfo_p.status();
                    //if status != ProcessStatus::Dead {//&& status != ProcessStatus::Stop {
                    res.push(p);
                    //}
                }
            }
        }
        debug!("End of get alive processes.");
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
        pid: i32, // the PID of the process to look for
        containers: &[Container],
        docker_version: String,
        pods: &[Pod],
        //kubernetes_version: String,
    ) -> HashMap<String, String> {
        let mut result = self.procs.iter().filter(
            // get all processes that have process records
            |x| !x.is_empty() && x.get(0).unwrap().process.pid == pid,
        );
        let process = result.next().unwrap();
        let mut description = HashMap::new();
        let regex_clean_container_id = Regex::new("[[:alnum:]]{12,}").unwrap();
        if let Some(p) = process.get(0) {
            // if we have the cgroups data from the original process struct
            if let Ok(cgroups) = p.process.original.cgroups() {
                let mut found = false;
                for cg in &cgroups {
                    if found {
                        break;
                    }
                    // docker
                    if self.regex_cgroup_docker.is_match(&cg.pathname) {
                        debug!("regex docker matched : {}", &cg.pathname); //coucou
                        description
                            .insert(String::from("container_scheduler"), String::from("docker"));
                        // extract container_id
                        //let container_id = cg.pathname.split('/').last().unwrap();
                        if let Some(container_id_capture) =
                            regex_clean_container_id.captures(&cg.pathname)
                        {
                            let container_id = &container_id_capture[0];
                            debug!("container_id = {}", container_id);
                            description
                                .insert(String::from("container_id"), String::from(container_id));
                            if let Some(container) =
                                containers.iter().find(|x| x.Id == container_id)
                            {
                                debug!("found container with id: {}", &container_id);
                                let mut names = String::from("");
                                for n in &container.Names {
                                    debug!("adding container name: {}", &n.trim().replace('/', ""));
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
                                            if let Some(final_id) = id.strip_prefix("docker://") {
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
                                description
                                    .insert(String::from("kubernetes_pod_name"), pod_name.clone());
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
        }
        description
    }

    /// Returns a vector containing pids of all running, sleeping or waiting current processes.
    pub fn get_alive_pids(&self) -> Vec<i32> {
        self.get_alive_processes()
            .iter()
            .filter(|x| !x.is_empty())
            .map(|x| x[0].process.pid)
            .collect()
    }

    /// Returns a vector containing pids of all processes being tracked.
    pub fn get_all_pids(&self) -> Vec<i32> {
        self.procs
            .iter()
            .filter(|x| !x.is_empty())
            .map(|x| x[0].process.pid)
            .collect()
    }

    /// Returns the process name associated to a PID
    pub fn get_process_name(&self, pid: i32) -> String {
        let mut result = self
            .procs
            .iter()
            .filter(|x| !x.is_empty() && x.get(0).unwrap().process.pid == pid);
        let process = result.next().unwrap();
        if result.next().is_some() {
            panic!("Found two vectors of processes with the same id, maintainers should fix this.");
        }
        debug!("End of get process name.");
        process.get(0).unwrap().process.comm.clone()
    }

    /// Returns the cmdline string associated to a PID
    pub fn get_process_cmdline(&self, pid: i32) -> Option<String> {
        let mut result = self
            .procs
            .iter()
            .filter(|x| !x.is_empty() && x.get(0).unwrap().process.pid == pid);
        let process = result.next().unwrap();
        if let Some(p) = process.get(0) {
            #[cfg(target_os = "windows")]
            let cmdline_request = p.process.cmdline(self);
            #[cfg(target_os = "linux")]
            let cmdline_request = p.process.cmdline();
            if let Ok(mut cmdline_vec) = cmdline_request {
                let mut cmdline = String::from("");
                while !cmdline_vec.is_empty() {
                    if !cmdline_vec.is_empty() {
                        cmdline.push_str(&cmdline_vec.remove(0));
                    }
                }
                debug!("End of get process cmdline.");
                return Some(cmdline);
            }
        }
        debug!("End of get process cmdline.");
        None
    }

    #[cfg(target_os = "linux")]
    /// Returns the CPU time consumed between two measure iteration
    fn get_cpu_time_consumed(&self, p: &[ProcessRecord]) -> u64 {
        let last_time = p.first().unwrap().total_time_jiffies();
        let previous_time = p.get(1).unwrap().total_time_jiffies();
        let mut diff = 0;
        if previous_time <= last_time {
            diff = last_time - previous_time;
        }
        diff
    }

    #[cfg(target_os = "windows")]
    pub fn get_cpu_usage_percentage(&self, pid: usize, nb_cores: usize) -> f32 {
        let mut cpu_current_usage = 0.0;
        for c in self.sysinfo.processors() {
            cpu_current_usage += c.cpu_usage();
        }
        if let Some(p) = self.sysinfo.process(pid) {
            (p.cpu_usage() + (100.0 - cpu_current_usage / nb_cores as f32) * p.cpu_usage() / 100.0)
                / nb_cores as f32
        } else {
            0.0
        }
    }

    /// Returns processes sorted by the highest consumers in first
    pub fn get_top_consumers(&self, top: u16) -> Vec<(IProcess, f64)> {
        let mut consumers: Vec<(IProcess, OrderedFloat<f64>)> = vec![];
        for p in &self.procs {
            if p.len() > 1 {
                #[cfg(target_os = "linux")]
                {
                    let diff = self.get_cpu_time_consumed(p);
                    if consumers
                        .iter()
                        .filter(|x| ProcessRecord::new(x.0.to_owned()).total_time_jiffies() > diff)
                        .count()
                        < top as usize
                    {
                        consumers
                            .push((p.last().unwrap().process.clone(), OrderedFloat(diff as f64)));
                        consumers.sort_by(|x, y| y.1.cmp(&x.1));
                        if consumers.len() > top as usize {
                            consumers.pop();
                        }
                    }
                }
                #[cfg(target_os = "windows")]
                {
                    let diff = self.get_cpu_usage_percentage(
                        p.first().unwrap().process.pid as _,
                        self.nb_cores,
                    );
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
                            let new_consumer = IProcess::from_windows_process(sysinfo_process);
                            consumers.push((new_consumer, OrderedFloat(diff as f64)));
                            consumers.sort_by(|x, y| y.1.cmp(&x.1));
                            if consumers.len() > top as usize {
                                consumers.pop();
                            }
                        } else {
                            warn!("Couldn't get process info for {}", pid);
                        }
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
                #[cfg(target_os = "linux")]
                {
                    let diff = self.get_cpu_time_consumed(p);
                    let process_exe = p.last().unwrap().process.exe().unwrap_or_default();
                    if regex_filter.is_match(process_exe.to_str().unwrap_or_default()) {
                        consumers
                            .push((p.last().unwrap().process.clone(), OrderedFloat(diff as f64)));
                        consumers.sort_by(|x, y| y.1.cmp(&x.1));
                    }
                }
                #[cfg(target_os = "windows")]
                {
                    let diff = self.get_cpu_usage_percentage(
                        p.first().unwrap().process.pid as _,
                        self.nb_cores,
                    );
                    let process_exe = p.last().unwrap().process.exe(self).unwrap_or_default();
                    if regex_filter.is_match(process_exe.to_str().unwrap_or_default()) {
                        consumers
                            .push((p.last().unwrap().process.clone(), OrderedFloat(diff as f64)));
                        consumers.sort_by(|x, y| y.1.cmp(&x.1));
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

    /// Drops a vector of ProcessRecord instances from self.procs
    /// if the last ProcessRecord from the vector is of state Terminated
    /// (if the process is not running anymore)
    pub fn clean_terminated_process_records_vectors(&mut self) {
        //TODO get stats from processes to know what is hapening !
        let mut d_unint_sleep = 0;
        let mut r_running = 0;
        let mut s_int_sleep = 0;
        let mut t_stopped = 0;
        let mut z_defunct_zombie = 0;
        let mut w_no_resident_high_prio = 0;
        let mut n_low_prio = 0;
        let mut l_pages_locked = 0;
        let mut i_idle = 0;
        let mut unknown = 0;
        for v in &mut self.procs {
            if !v.is_empty() {
                if let Some(first) = v.first() {
                    if let Ok(status) = first.process.status() {
                        if status.state.contains('T') {
                            while !v.is_empty() {
                                v.pop();
                            }
                            t_stopped += 1;
                        } else if status.state.contains('D') {
                            d_unint_sleep += 1;
                        } else if status.state.contains('R') {
                            r_running += 1;
                        } else if status.state.contains('S') {
                            s_int_sleep += 1;
                        } else if status.state.contains('Z') {
                            z_defunct_zombie += 1;
                        } else if status.state.contains('W') {
                            w_no_resident_high_prio += 1;
                        } else if status.state.contains('N') {
                            n_low_prio += 1;
                        } else if status.state.contains('L') {
                            l_pages_locked += 1;
                        } else if status.state.contains('I') {
                            i_idle += 1;
                        } else {
                            unknown += 1;
                            debug!("unkown state: {} name: {}", status.state, status.name);
                        }
                    } else {
                        while !v.is_empty() {
                            v.pop();
                        }
                    }
                }
            }
        }
        debug!(
            "d:{} r:{} s:{} t:{} z:{} w:{} n:{} l:{} i:{} u:{}",
            d_unint_sleep,
            r_running,
            s_int_sleep,
            t_stopped,
            z_defunct_zombie,
            w_no_resident_high_prio,
            n_low_prio,
            l_pages_locked,
            i_idle,
            unknown
        );
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

    // Returns the total CPU time consumed by this process since its creation
    pub fn total_time_jiffies(&self) -> u64 {
        #[cfg(target_os = "linux")]
        if let Some(stat) = &self.process.stat {
            trace!(
                "ProcessRecord: stime {} utime {}", //cutime {} cstime {} guest_time {} cguest_time {} delayacct_blkio_ticks {} itrealvalue {}",
                stat.stime,
                stat.utime //, cutime, cstime, guest_time, cguest_time, delayacct_blkio_ticks, itrealvalue
            );
            return stat.stime + stat.utime;
        } else {
            warn!("No IStat !");
        }

        //#[cfg(target_os="windows")]
        //let usage = &self.sysinfo.
        //let cutime = self.process.stat.cutime as u64;
        //let cstime = self.process.stat.cstime as u64;
        //let guest_time = self.process.stat.guest_time.unwrap_or_default();
        //let cguest_time = self.process.stat.cguest_time.unwrap_or_default() as u64;
        //let delayacct_blkio_ticks = self.process.stat.delayacct_blkio_ticks.unwrap_or_default();
        //let itrealvalue = self.process.stat.itrealvalue as u64;

        // not including cstime and cutime in total as they are reported only when child dies
        // child metrics as already reported as the child processes are in the global process
        // list, found as /proc/PID/stat
        0 //+ guest_time + cguest_time + delayacct_blkio_ticks + itrealvalue
    }
}

/// Returns a Duration instance with the current timestamp
pub fn current_system_time_since_epoch() -> Duration {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    #[test]
    fn process_records_added() {
        let proc = Process::myself().unwrap();
        let mut tracker = ProcessTracker::new(3);
        for _ in 0..3 {
            assert_eq!(
                tracker
                    .add_process_record(IProcess::from_linux_process(&proc))
                    .is_ok(),
                true
            );
        }
        assert_eq!(tracker.procs.len(), 1);
        assert_eq!(tracker.procs[0].len(), 3);
    }

    #[test]
    fn process_records_cleaned() {
        let proc = Process::myself().unwrap();
        let mut tracker = ProcessTracker::new(3);
        for _ in 0..5 {
            assert_eq!(
                tracker
                    .add_process_record(IProcess::from_linux_process(&proc))
                    .is_ok(),
                true
            );
        }
        assert_eq!(tracker.procs.len(), 1);
        assert_eq!(tracker.procs[0].len(), 3);
        for _ in 0..15 {
            assert_eq!(
                tracker
                    .add_process_record(IProcess::from_linux_process(&proc))
                    .is_ok(),
                true
            );
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
