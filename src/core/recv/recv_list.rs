use std::{collections::HashMap, time::Duration};

use crate::{
    core::loss_list::LossBuffer,
    packet::{control::ack::Ack, data::DataPacket},
    utils::{MessageNumber, SequenceNumber, SequenceRange},
    window::time_window::TimeWindow,
};

use super::recv_buffer::RecvBuffer;

pub struct RecvList {
    connections: HashMap<u16, RecvBacker>,
}
struct RecvBacker {
    data_buffer: RecvBuffer,
    loss_buffer: LossBuffer,
    time_window: TimeWindow,
}

impl RecvList {
    pub fn new() -> Self {
        let connections = HashMap::new();
        Self { connections }
    }
    pub fn register_connection(&mut self, socket_id: u16, self_isn: SequenceNumber,partner_isn: SequenceNumber) {
        let data_buffer = RecvBuffer::new(self_isn,partner_isn);
        let loss_buffer = LossBuffer::new();
        let time_window = TimeWindow::new();
        let backer = RecvBacker {
            data_buffer,
            loss_buffer,
            time_window,
        };
        self.connections.insert(socket_id, backer);
    }
    pub fn remove_connection(&mut self, socket_id: u16) {
        self.connections.remove(&socket_id);
    }
    pub fn add_data(&mut self, packet: DataPacket) {
        let socket_id = packet.dst_socket_id;
        let seq_no = packet.seq_no;
        match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                //time handling
                connection.time_window.on_packet_arrival();
                if packet.seq_no.probe_start() {
                    connection.time_window.probe_start();
                } else if packet.seq_no.probe_stop() {
                    connection.time_window.probe_stop();
                }
                //data handling
                let skip_range = connection.data_buffer.add(packet);
                connection.loss_buffer.remove(seq_no);

                //loss handling
                match skip_range {
                    Some(skip) => connection.loss_buffer.insert(skip),
                    None => {}
                };
            }
            None => {}
        }
    }
    pub fn report_loss(&mut self, socket_id: u16, mss: u16) -> Vec<SequenceRange> {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => connection.loss_buffer.encode(mss),
            None => vec![],
        }
    }
    pub fn pop_data(&mut self, socket_id: u16) -> Option<Vec<u8>> {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => connection.data_buffer.pop(),
            None => None,
        }
    }
    pub fn drop_msg(&mut self, socket_id: u16, msg_no: MessageNumber, range: SequenceRange) {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => connection.data_buffer.drop_msg(msg_no, range),
            None => {}
        }
    }
    pub fn recv_speed(&self, socket_id: u16) -> usize {
        match self.connections.get(&socket_id) {
            Some(connection) => connection.time_window.receive_speed(),
            None => 0,
        }
    }
    pub fn bandwidth(&self, socket_id: u16) -> usize {
        match self.connections.get(&socket_id) {
            Some(connection) => connection.time_window.bandwidth(),
            None => 0,
        }
    }
    pub fn buffer_size(&self, socket_id: u16) -> usize {
        match self.connections.get(&socket_id) {
            Some(connection) => connection.data_buffer.size(),
            None => 0,
        }
    }
    pub fn update_delay(&mut self, socket_id: u16, factor: f64) {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                connection.time_window.update_delay(factor);
            }
            None => {}
        }
    }
    pub fn next_ack(&mut self, socket_id: u16) -> Option<(SequenceNumber,SequenceNumber)> {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                let proposed_ack = match connection.loss_buffer.first() {
                    Some(range) =>range.start,
                    None => connection.data_buffer.next_ack()
                };
                connection.data_buffer.should_ack(proposed_ack)
            }
            None => None,
        }
    }
    pub fn sent_ack(&mut self, socket_id: u16,ack_no: SequenceNumber) {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => connection.data_buffer.sent_ack(ack_no),
            None => {},
        }
    }
    pub fn ack_square(&mut self, socket_id: u16, ack_no: SequenceNumber) {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => connection.data_buffer.ack_square(ack_no),
            None => {}
        }
    }
    pub fn loss(&mut self, socket_id: u16, loss_ranges: Vec<SequenceRange>) {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => connection.data_buffer.loss(loss_ranges),
            None => {}
        }
    }
    pub fn on_pkt(&mut self, socket_id: u16) {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => connection.data_buffer.on_pkt(),
            None => {}
        }
    }
    pub fn delay(&self, socket_id: u16) -> Duration {
        match self.connections.get(&socket_id) {
            Some(connection) => connection.data_buffer.delay(),
            None => Duration::ZERO,
        }
    }
    pub fn on_ack(&mut self, socket_id: u16,ack_no: SequenceNumber, ack:Ack){
        match self.connections.get_mut(&socket_id){
            Some(connection) => {
                connection.data_buffer.update_rtt(ack.rtt);
                connection.data_buffer.update_recv_rate(ack.window);
                connection.data_buffer.update_bandwidth(ack.bandwidth);
                connection.data_buffer.on_ack(ack_no);

            },
            None => {},
        }
    }
    pub fn rtt(&self,socket_id: u16)->(Duration,Duration){
        match self.connections.get(&socket_id){
            Some(connection) => {
                connection.data_buffer.rtt()

            },
            None => (Duration::ZERO,Duration::ZERO),
        }
    }
}
