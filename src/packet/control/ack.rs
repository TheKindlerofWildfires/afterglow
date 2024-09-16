use crate::{serial::Serial, utils::SequenceNumber};

#[derive(Clone, Debug)]
pub struct Ack {
    pub seq_no: SequenceNumber,
    pub rtt: u16,
    pub rrt_variance: u16,
    pub buffer_size: u16,
    pub window: u16,
    pub bandwidth: u16,
}

impl Ack {
    pub fn new(
        seq_no:SequenceNumber,
        rtt: u16,
        rrt_variance: u16,
        buffer_size: u16,
        window: u16,
        bandwidth: u16,
    ) -> Self {
        Self {
            seq_no,
            rtt,
            rrt_variance,
            buffer_size,
            window,
            bandwidth,
        }
    }
}

impl Serial for Ack {
    fn serialize(&self) -> Vec<u8> {
        let mut bytes = self.seq_no.serialize();
        bytes.extend_from_slice(&self.rtt.serialize());
        bytes.extend_from_slice(&self.rrt_variance.serialize());
        bytes.extend_from_slice(&self.buffer_size.serialize());
        bytes.extend_from_slice(&self.window.serialize());
        bytes.extend_from_slice(&self.bandwidth.serialize());
        bytes
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let seq_no = SequenceNumber::deserialize(bytes, start);
        let rtt = u16::deserialize(bytes, start);
        let rrt_variance = u16::deserialize(bytes, start);
        let buffer_size = u16::deserialize(bytes, start);
        let window = u16::deserialize(bytes, start);
        let bandwidth = u16::deserialize(bytes, start);
        Self {
            seq_no,
            rtt,
            rrt_variance,
            buffer_size,
            window,
            bandwidth,
        }
    }
}
