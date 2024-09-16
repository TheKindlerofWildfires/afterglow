use std::time::{Duration, SystemTime};

use crate::utils::SequenceNumber;


#[derive(Clone,Debug)]
pub struct Window {
    acks: Vec<WindowAck>,
    timeout: Duration,
}

#[derive(Copy,Clone,Debug)]
pub struct WindowAck {
    seq: SequenceNumber,
    ack: SequenceNumber,
    time: SystemTime,
}

impl WindowAck {
    pub fn new(seq: SequenceNumber, ack: SequenceNumber) -> Self {
        let time = SystemTime::now();
        Self { seq, ack, time }
    }
}

impl Window {
    pub fn new(timeout: Duration) -> Self {
        let acks = Vec::new();
        Self { acks, timeout }
    }
    pub fn store(&mut self, seq: SequenceNumber, ack: SequenceNumber) {
        //keep only acks within timeout
        self.acks.retain(|ack| match ack.time.elapsed() {
            Ok(elapsed) => elapsed < self.timeout,
            Err(_) => false,
        });
        //add the new ack
        self.acks.push(WindowAck::new(seq, ack));
    }
    pub fn acknowledge(&mut self, seq: SequenceNumber, ack: &mut SequenceNumber) -> Duration {
        //find the matching seq
        let mut time = Duration::ZERO;

        //This actually removes all matching sequence numbers and gives the temporally first ack
        self.acks.retain(|window_ack| {
            let seq_match = window_ack.seq == seq;
            if seq_match {
                let proposed_time = match window_ack.time.elapsed() {
                    Ok(time) => time,
                    Err(_) => Duration::MAX,
                };
                if proposed_time > time {
                    time = proposed_time;
                    *ack = window_ack.ack;
                }
            }
            !seq_match
        });
        time
    }
}
