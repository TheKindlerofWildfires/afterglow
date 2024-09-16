use std::{collections::HashMap, time::Duration};

use crate::{
    core::loss_list::LossBuffer, packet::data::DataPacket, utils::{MessageNumber, SequenceNumber, SequenceRange}, window::time_window::TimeWindow
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
    pub fn register_connection(&mut self, socket_id: u16, isn: SequenceNumber) {
        let data_buffer = RecvBuffer::new(isn);
        let loss_buffer = LossBuffer::new();
        let time_window = TimeWindow::new();
        let backer = RecvBacker {
            data_buffer,
            loss_buffer,
            time_window,
        };
        self.connections.insert(socket_id, backer);
    }
    pub fn remove_connection(&mut self, socket_id: u16){
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
    pub fn report_loss(
        &mut self,
        socket_id: u16,
        mss: u16,
    ) -> Vec<SequenceRange> {
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
    pub fn recv_speed(&self, socket_id: u16) -> Duration {
        match self.connections.get(&socket_id) {
            Some(connection) => connection.time_window.receive_speed(),
            None => Duration::ZERO,
        }
    }
    pub fn bandwidth(&self, socket_id: u16) -> Duration {
        match self.connections.get(&socket_id) {
            Some(connection) => connection.time_window.bandwidth(),
            None => Duration::ZERO,
        }
    }
    pub fn buffer_size(&self, socket_id: u16)->u16{
        match self.connections.get(&socket_id) {
            Some(connection) => connection.data_buffer.size() as u16,
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
    pub fn next_ack(&mut self, socket_id: u16) -> Option<SequenceNumber>{
        match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                match connection.loss_buffer.first(){
                    Some(range) => Some(range.start),
                    None => {
                        let mut ack_no = connection.data_buffer.last_seq();
                        ack_no.inc();
                        Some(ack_no)
                    },
                }
            }
            None => None
        }

    }
    pub fn inc_ack(&mut self, socket_id: u16) -> Option<SequenceNumber>{
        match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                match connection.loss_buffer.first(){
                    Some(range) => Some(range.start),
                    None => {
                        let ack_no = connection.data_buffer.inc_ack();
                        Some(ack_no)
                    },
                }
            }
            None => None
        }

    }
}
