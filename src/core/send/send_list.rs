use std::{
    collections::{BinaryHeap, HashMap},
    time::{Duration, SystemTime},
};

use crate::{
    core::loss_list::LossBuffer,
    packet::Packet,
    utils::{MessageNumber, SequenceNumber, SequenceRange},
};

use super::send_buffer::SendBuffer;

pub struct SendList {
    connections: HashMap<u16, SendBacker>,
}
#[derive(Debug)]
struct SendBacker {
    data_buffer: SendBuffer,
    loss_buffer: LossBuffer,
    updates: BinaryHeap<SystemTime>,
}

impl SendList {
    pub fn new() -> Self {
        let connections = HashMap::new();
        Self { connections }
    }
    pub fn register_connection(&mut self, socket_id: u16, isn: SequenceNumber) {
        let data_buffer = SendBuffer::new(isn);
        let loss_buffer = LossBuffer::new();
        let updates = BinaryHeap::new();
        let backer = SendBacker {
            data_buffer,
            loss_buffer,
            updates,
        };
        self.connections.insert(socket_id, backer);
    }
    pub fn remove_connection(&mut self, socket_id: u16) {
        self.connections.remove(&socket_id);
    }
    //Wrapper for send buffer add
    pub fn insert(
        &mut self,
        socket_id: u16,
        data: &[u8],
        ttl: Duration,
        order: bool,
        partner_id: u16,
        mss: u16,
    ) ->usize {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => connection
                .data_buffer
                .add(data, ttl, order, partner_id, mss),
            None => 0
        }
    }
    //Wrapper for updating time
    pub fn update(&mut self, socket_id: u16, time: SystemTime) {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                connection.updates.push(time);
            }
            None => {}
        }
    }
    pub fn drops(&mut self, socket_id: u16) -> HashMap<MessageNumber, Vec<SequenceNumber>> {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => connection.data_buffer.drop(),
            None => HashMap::new(),
        }
    }

    pub fn pop(&mut self, socket_id: u16) -> Option<Packet> {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                while let Some(loss) = connection.loss_buffer.first() {
                    match connection.data_buffer.search(loss) {
                        Some(packet) => {
                            connection.loss_buffer.remove(packet.seq_no);
                            return Some(Packet::Data(packet));
                        }
                        None => {
                            connection.loss_buffer.pop();
                        }
                    }
                }
                match connection.data_buffer.read() {
                    Some(packet) => Some(Packet::Data(packet)),
                    None => None,
                }
            }
            None => None,
        }
    }

    pub fn loss(&mut self, socket_id: u16, loss: SequenceRange) {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                if loss.start < loss.stop && loss.stop < connection.data_buffer.last_seq() {
                    connection.loss_buffer.insert(loss);
                }
            }
            None => {}
        }
    }
    pub fn out_of_sequence(&mut self, socket_id: u16, ack_no: SequenceNumber) -> bool {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => connection.data_buffer.last_seq() < ack_no,
            None => false,
        }
    }
    pub fn remove_confirmed(&mut self, socket_id: u16, ack_no: SequenceNumber) {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                connection.loss_buffer.remove_confirmed(ack_no);
            }
            None => {}
        }
    }

    pub fn next_time(&mut self) -> Option<(u16, Duration)> {
        let result = self
            .connections
            .iter()
            .fold(None, |acc, (c_sock, c_conn)| match acc {
                Some((_, acc_delay)) => {
                    match c_conn.updates.peek() {
                        Some(time) => match time.duration_since(SystemTime::now()) {
                            Ok(delay) => {
                                if delay < acc_delay {
                                    Some((*c_sock, delay))
                                } else {
                                    acc
                                }
                            }
                            Err(_) => Some((*c_sock, Duration::ZERO)),
                        },
                        None => acc,
                    }
                }
                None => {
                    match c_conn.updates.peek() {
                    Some(time) => match time.duration_since(SystemTime::now()) {
                        Ok(delay) => Some((*c_sock, delay)),
                        Err(_) => Some((*c_sock, Duration::ZERO)),
                    },
                    None => None,
                }},
            });
        match result {
            Some((socket_id, delay)) => {
                let delay = match self.pop_time(socket_id) {
                    Some(delay) => delay,
                    None => delay,
                };
                Some((socket_id, delay))
            }
            None => None,
        }
    }
    pub fn pop_time(&mut self, socket_id: u16) -> Option<Duration> {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => match connection.updates.pop() {
                Some(time) => match time.duration_since(SystemTime::now()) {
                    Ok(delay) => Some(delay),
                    Err(_) => None,
                },
                None => None,
            },
            None => None,
        }
    }
}
