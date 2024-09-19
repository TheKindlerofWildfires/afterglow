use std::time::{Duration, SystemTime};
pub struct TimeWindow {
    min_send_interval: Duration,
    last_packet: SystemTime,
    last_probe: SystemTime,
    packet_windows: Vec<usize>,
    probe_windows: Vec<usize>,
    packet_size: usize,
    probe_size: usize,
    packet_index: usize,
    probe_index: usize,
}
const MEDIAN_LIMIT: usize = 128;

impl TimeWindow {
    pub fn new() -> Self {
        let packet_size = 16;
        let probe_size = 64;
        let last_packet = SystemTime::now();
        let last_probe = SystemTime::now();
        let min_send_interval = Duration::from_millis(100);
        let packet_windows = Vec::with_capacity(packet_size);
        let probe_windows = Vec::with_capacity(probe_size);
        let packet_index = 0;
        let probe_index = 0;
        Self {
            min_send_interval,
            last_packet,
            last_probe,
            packet_windows,
            probe_windows,
            packet_size,
            probe_size,
            packet_index,
            probe_index,
        }
    }
    pub fn update_delay(&mut self, factor: f64){
        self.min_send_interval =
            Duration::from_micros((self.min_send_interval.as_micros() as f64 * factor) as u64);
    }
    pub fn receive_speed(&self) -> usize {
        if self.packet_windows.len()==0{
            return 0
        }
        let mut replica = self.packet_windows.clone();
        replica.sort_unstable();
        let median = replica[replica.len() / 2];

        let upper = median * 8;
        let lower = median / 8;

        //divergent: used the sorted array to search faster
        let start = replica
            .iter()
            .position(|rep| *rep >= lower)
            .expect("Invariant in median failed");
        let stop = replica
            .iter()
            .position(|rep| *rep <= upper)
            .expect("Invariant in median failed");
        let count = stop - start;
        if count < MEDIAN_LIMIT {
            0
        } else {
            let sum = replica[start..stop]
                .iter()
                .fold(0, |acc, rep| acc + *rep);
            sum / count
        }
    }
    pub fn bandwidth(&self) -> usize {
        if self.probe_windows.len()==0{
            return 0
        }
        let mut replica = self.probe_windows.clone();
        replica.sort_unstable();
        let median = replica[replica.len() / 2];

        let upper = median * 8;
        let lower = median / 8;

        //divergent: used the sorted array to search faster
        let start = replica
            .iter()
            .position(|rep| *rep >= lower)
            .expect("Invariant in median failed");
        let stop = replica
            .iter()
            .position(|rep| *rep <= upper)
            .expect("Invariant in median failed");
        let count = stop - start;
        if count < MEDIAN_LIMIT {
            0
        } else {
            let sum = replica[start..stop]
                .iter()
                .fold(0, |acc, rep| acc + *rep);
            sum / count
        }
    }
    pub fn on_packet_sent(&mut self, send_time: SystemTime) {
        match send_time.duration_since(self.last_packet) {
            Ok(interval) => {
                self.min_send_interval = interval;
            }
            Err(_) => {}
        };
        self.last_packet = send_time;
    }
    pub fn on_packet_arrival(&mut self) {
        let time = SystemTime::now();
        match time.duration_since(self.last_packet) {
            Ok(duration) => {
                let dur = duration.as_millis() as usize;
                if self.packet_windows.len() < self.packet_size {
                    self.packet_windows.push(dur)
                } else {
                    self.packet_windows[self.packet_index] = dur;
                    self.packet_index = (self.packet_index + 1) % self.packet_size;
                }
            }
            Err(_) => {}
        }
    }
    pub fn probe_start(&mut self) {
        self.last_probe = SystemTime::now();
    }
    pub fn probe_stop(&mut self) {
        let time = SystemTime::now();
        match time.duration_since(self.last_packet) {
            Ok(duration) => {
                let dur = duration.as_millis() as usize;
                if self.probe_windows.len() < self.probe_size {
                    self.probe_windows.push(dur)
                } else {
                    self.probe_windows[self.probe_index] = dur;
                    self.probe_index = (self.probe_index + 1) % self.probe_size;
                }
            }
            Err(_) => {}
        }
    }
}
