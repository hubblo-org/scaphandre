use core::fmt::Debug;
use std::error::Error;
use std::mem::size_of_val;
use dyn_clone::DynClone;

use crate::sensors::units;
use super::{Record, Domain, CPUStat, CPUCore, RecordGenerator, StatsGenerator};

pub trait Socket: DynClone + Send {
    fn read_record_uj(&self) -> Result<Record, Box<dyn Error>>;
    fn get_record_buffer(&mut self) -> &mut Vec<Record>;
    fn get_record_buffer_passive(&self) -> &Vec<Record>;
    fn get_buffer_max_kbytes(&self) -> u16;
    fn get_id(&self) -> u16;
    fn get_domains_passive(&self) -> &Vec<Domain>;
    fn get_domains(&mut self) -> &mut Vec<Domain>;
    fn get_stat_buffer(&mut self) -> &mut Vec<CPUStat>;
    fn get_stat_buffer_passive(&self) -> &Vec<CPUStat>;
    fn read_stats(&self) -> Option<CPUStat>;
    fn get_cores(&mut self) -> &mut Vec<CPUCore>;
    fn get_cores_passive(&self) -> &Vec<CPUCore>;
    fn get_debug_type(&self) -> String;
}

dyn_clone::clone_trait_object!(Socket);

impl dyn Socket {
    pub fn add_cpu_core(&mut self, core: CPUCore) {
        self.get_cores().push(core);
    }

    /// Adds a new Domain instance to the domains vector if and only if it doesn't exist in the vector already.
    pub fn safe_add_domain(&mut self, domain: Domain) {
        let domains = self.get_domains();
        if !domains.iter().any(|d| d.id == domain.id) {
            domains.push(domain);
        }
    }

    /// Returns a Record instance containing the power consumed between last
    /// and previous measurement, for this CPU socket
    pub fn get_records_diff_power_microwatts(&self) -> Option<Record> {
        let record_buffer = self.get_record_buffer_passive();
        if record_buffer.len() > 1 {
            let last_record = record_buffer.last().unwrap();
            let previous_record = record_buffer
                .get(record_buffer.len() - 2)
                .unwrap();
            debug!(
                "last_record value: {} previous_record value: {}",
                &last_record.value, &previous_record.value
            );
            let last_rec_val = last_record.value.trim();
            debug!("l851 : trying to parse {} as u64", last_rec_val);
            let prev_rec_val = previous_record.value.trim();
            debug!("l853 : trying to parse {} as u64", prev_rec_val);
            if let (Ok(last_microjoules), Ok(previous_microjoules)) =
                (last_rec_val.parse::<u64>(), prev_rec_val.parse::<u64>())
            {
                let mut microjoules = 0;
                if last_microjoules >= previous_microjoules {
                    microjoules = last_microjoules - previous_microjoules;
                } else {
                    debug!(
                        "previous_microjoules ({}) > last_microjoules ({})",
                        previous_microjoules, last_microjoules
                    );
                }
                let time_diff =
                    last_record.timestamp.as_secs_f64() - previous_record.timestamp.as_secs_f64();
                let microwatts = microjoules as f64 / time_diff;
                debug!("l866: microwatts: {}", microwatts);
                return Some(Record::new(
                    last_record.timestamp,
                    (microwatts as u64).to_string(),
                    units::Unit::MicroWatt,
                ));
            }
        } else {
            debug!("Not enough records for socket");
        }
        None
    }
}

impl RecordGenerator for dyn Socket {
    fn refresh_record(&mut self) {
        if let Ok(record) = self.read_record_uj() {
            self.get_record_buffer().push(record);
        }

        if !self.get_record_buffer().is_empty() {
            self.clean_old_records();
        }
    }

    fn clean_old_records(&mut self) {
        let buffer_max_kbytes = self.get_buffer_max_kbytes();
        let id = self.get_id();
        let record_buffer = self.get_record_buffer();
        let record_ptr = &record_buffer[0];
        let curr_size = size_of_val(record_ptr) * record_buffer.len();
        trace!(
            "socket rebord buffer current size: {} max_bytes: {}",
            curr_size,
            buffer_max_kbytes * 1000
        );
        if curr_size > (buffer_max_kbytes * 1000) as usize {
            let size_diff = curr_size - (buffer_max_kbytes * 1000) as usize;
            trace!(
                "socket record size_diff: {} sizeof: {}",
                size_diff,
                size_of_val(record_ptr)
            );
            if size_diff > size_of_val(record_ptr) {
                let nb_records_to_delete = size_diff as f32 / size_of_val(record_ptr) as f32;
                for _ in 1..nb_records_to_delete as u32 {
                    if !record_buffer.is_empty() {
                        let res =record_buffer.remove(0);
                        debug!(
                            "Cleaning socket id {} records buffer, removing: {}",
                            id, res
                        );
                    }
                }
            }
        }
    }

    fn get_records_passive(&self) -> Vec<Record> {
        let mut result = vec![];
        for r in self.get_record_buffer_passive() {
            result.push(Record::new(
                r.timestamp,
                r.value.clone(),
                units::Unit::MicroJoule,
            ));
        }
        result
    }
}

impl StatsGenerator for dyn Socket {
    /// Generates a new CPUStat object storing current usage statistics of the socket
    /// and stores it in the stat_buffer.
    fn refresh_stats(&mut self) {
        let stat_buffer = self.get_stat_buffer_passive();
        if !stat_buffer.is_empty() {
            self.clean_old_stats();
        }
        let stats = self.read_stats();
        self.get_stat_buffer().insert(0, stats.unwrap());
    }

    /// Checks the size in memory of stats_buffer and deletes as many CPUStat
    /// instances from the buffer to make it smaller in memory than buffer_max_kbytes.
    fn clean_old_stats(&mut self) {
        let id = self.get_id();
        let buffer_max_kbytes = self.get_buffer_max_kbytes();
        let stat_buffer = self.get_stat_buffer();
        let stat_ptr = &stat_buffer[0];
        let size_of_stat = size_of_val(stat_ptr);
        let curr_size = size_of_stat * stat_buffer.len();
        trace!("current_size of stats in socket {}: {}", id, curr_size);
        trace!(
            "estimated max nb of socket stats: {}",
            buffer_max_kbytes as f32 * 1000.0 / size_of_stat as f32
        );
        if curr_size > (buffer_max_kbytes * 1000) as usize {
            let size_diff = curr_size - (buffer_max_kbytes * 1000) as usize;
            trace!(
                "socket {} size_diff: {} size of: {}",
                id,
                size_diff,
                size_of_stat
            );
            if size_diff > size_of_stat {
                let nb_stats_to_delete = size_diff as f32 / size_of_stat as f32;
                trace!(
                    "socket {} nb_stats_to_delete: {} size_diff: {} size of: {}",
                    id,
                    nb_stats_to_delete,
                    size_diff,
                    size_of_stat
                );
                trace!("nb stats to delete: {}", nb_stats_to_delete as u32);
                for _ in 1..nb_stats_to_delete as u32 {
                    if !stat_buffer.is_empty() {
                        let res = stat_buffer.pop();
                        debug!(
                            "Cleaning stat buffer of socket {}, removing: {:?}",
                            id, res
                        );
                    }
                }
            }
        }
    }


    /// Computes the difference between previous usage statistics record for the socket
    /// and the current one. Returns a CPUStat object containing this difference, field
    /// by field.
    fn get_stats_diff(&mut self) -> Option<CPUStat> {
        let stat_buffer = self.get_stat_buffer();
        if stat_buffer.len() > 1 {
            let last = &stat_buffer[0];
            let previous = &stat_buffer[1];
            let mut iowait = None;
            let mut irq = None;
            let mut softirq = None;
            let mut steal = None;
            let mut guest = None;
            let mut guest_nice = None;
            if last.iowait.is_some() && previous.iowait.is_some() {
                iowait = Some(last.iowait.unwrap() - previous.iowait.unwrap());
            }
            if last.irq.is_some() && previous.irq.is_some() {
                irq = Some(last.irq.unwrap() - previous.irq.unwrap());
            }
            if last.softirq.is_some() && previous.softirq.is_some() {
                softirq = Some(last.softirq.unwrap() - previous.softirq.unwrap());
            }
            if last.steal.is_some() && previous.steal.is_some() {
                steal = Some(last.steal.unwrap() - previous.steal.unwrap());
            }
            if last.guest.is_some() && previous.guest.is_some() {
                guest = Some(last.guest.unwrap() - previous.guest.unwrap());
            }
            if last.guest_nice.is_some() && previous.guest_nice.is_some() {
                guest_nice = Some(last.guest_nice.unwrap() - previous.guest_nice.unwrap());
            }
            return Some(CPUStat {
                user: last.user - previous.user,
                nice: last.nice - previous.nice,
                system: last.system - previous.system,
                idle: last.idle - previous.idle,
                iowait,
                irq,
                softirq,
                steal,
                guest,
                guest_nice,
            });
        }
        None
    }
}

impl Debug for dyn Socket {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} Socket (id={})", self.get_debug_type(), self.get_id())
    }
}