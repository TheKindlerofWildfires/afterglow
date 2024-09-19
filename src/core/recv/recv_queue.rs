use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use crate::{
    packet::{control::ack::Ack, data::DataPacket},
    utils::{MessageNumber, SequenceNumber, SequenceRange},
};

use super::recv_list::RecvList;

pub struct RecvQueue {
    list: Arc<RwLock<RecvList>>,
}

/*
    This takes inbound packets via the channel and puts them into the spot
    It also exposes a place for streams to take packets from (via core)
    It's also responsible for exposing the necessary elements of control functionality to core

    Proposal: this is mostly responsible for taking inbound packets and
        Handling control
        Passing data to the recv buffer
        passing managed results from the buffer to the stream

*/
impl RecvQueue {
    pub fn new() -> Self {
        let list = Arc::new(RwLock::new(RecvList::new()));
        Self { list }
    }
    pub fn register_connection(
        &self,
        socket_id: u16,
        self_isn: SequenceNumber,
        partner_isn: SequenceNumber,
    ) {
        match self.list.write() {
            Ok(mut binding) => binding.register_connection(socket_id, self_isn, partner_isn),
            Err(_) => {}
        }
    }
    pub fn drop_msg(&self, socket_id: u16, msg_no: MessageNumber, range: SequenceRange) {
        match self.list.write() {
            Ok(mut binding) => binding.drop_msg(socket_id, msg_no, range),
            Err(_) => {}
        }
    }

    pub fn process_data(&mut self, packet: DataPacket, mss: u16) -> Vec<SequenceRange> {
        match self.list.write() {
            Ok(mut binding) => {
                let socket_id = packet.dst_socket_id;
                //add the data
                binding.add_data(packet);
                //trigger an up to date loss list
                binding.report_loss(socket_id, mss)
            }
            Err(_) => {
                vec![]
            }
        }
    }

    pub fn update_delay(&self, socket_id: u16, factor: f64) {
        match self.list.write() {
            Ok(mut binding) => binding.update_delay(socket_id, factor),
            Err(_) => {}
        }
    }
    pub fn next_ack(&self, socket_id: u16) -> Option<(SequenceNumber, SequenceNumber)> {
        match self.list.write() {
            Ok(mut binding) => binding.next_ack(socket_id),
            Err(_) => None,
        }
    }
    pub fn sent_ack(&self, socket_id: u16, ack_no: SequenceNumber) {
        match self.list.write() {
            Ok(mut binding) => binding.sent_ack(socket_id, ack_no),
            Err(_) => {}
        }
    }

    pub fn time_data(&self, socket_id: u16) -> Option<(Duration, Duration, usize, usize, usize)> {
        match self.list.read() {
            Ok(binding) => {
                let (rtt, rtt_var) = binding.rtt(socket_id);
                Some((
                    rtt,
                    rtt_var,
                    binding.buffer_size(socket_id),
                    binding.recv_speed(socket_id),
                    binding.bandwidth(socket_id),
                ))
            }
            Err(_) => None,
        }
    }

    pub fn remove(&mut self, socket_id: u16) {
        match self.list.write() {
            Ok(mut binding) => binding.remove_connection(socket_id),
            Err(_) => {}
        }
    }
    pub fn read_data(&self, socket_id: u16) -> Option<Vec<u8>> {
        match self.list.write() {
            Ok(mut binding) => binding.pop_data(socket_id),
            Err(_) => None,
        }
    }
    pub fn loss(&self, socket_id: u16, loss_ranges: Vec<SequenceRange>) {
        match self.list.write() {
            Ok(mut binding) => binding.loss(socket_id, loss_ranges),
            Err(_) => {}
        }
    }
    pub fn ack_square(&mut self, socket_id: u16, ack_no: SequenceNumber) {
        match self.list.write() {
            Ok(mut list) => {
                list.ack_square(socket_id, ack_no);
            }
            Err(_) => {}
        }
    }
    pub fn delay(&self, socket_id: u16) -> Duration {
        match self.list.read() {
            Ok(list) => list.delay(socket_id),
            Err(_) => Duration::ZERO,
        }
    }
    pub fn on_pkt(&mut self, socket_id: u16) {
        match self.list.write() {
            Ok(mut list) => {
                list.on_pkt(socket_id);
            }
            Err(_) => {}
        }
    }
    pub fn on_ack(&mut self, socket_id: u16, ack_no: SequenceNumber, ack: Ack) {
        match self.list.write() {
            Ok(mut list) => {
                list.on_ack(socket_id, ack_no, ack);
            }
            Err(_) => {}
        }
    }
}
