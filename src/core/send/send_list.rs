use std::{
    collections::{BinaryHeap, HashMap},
    sync::{Arc, Condvar, Mutex},
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
    poll: Arc<(Condvar, Arc<Mutex<Vec<u16>>>)>,
}
#[derive(Debug)]
struct SendBacker {
    data_buffer: SendBuffer,
    loss_buffer: LossBuffer,
    updates: BinaryHeap<SystemTime>,
}

impl Default for SendList {
    fn default() -> Self {
        Self::new()
    }
}

impl SendList {
    pub fn new() -> Self {
        let connections = HashMap::new();
        let waiting_sockets = Arc::new(Mutex::new(Vec::new()));
        let work_condition = Condvar::new();
        let poll = Arc::new((work_condition, waiting_sockets));
        Self { connections, poll }
    }
    pub fn register_connection(
        &mut self,
        socket_id: u16,
        self_isn: SequenceNumber,
    ) {
        let data_buffer = SendBuffer::new(self_isn);
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
    ) -> usize {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => connection
                .data_buffer
                .add(data, ttl, order, partner_id, mss),
            None => 0,
        }
    }
    //Wrapper for updating time
    pub fn update(&mut self, socket_id: u16, cnt: usize, delay: Duration) {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                (0..cnt).for_each(|i| {
                    connection
                        .updates
                        .push(SystemTime::now() + delay * (i + 1) as u32);
                });
                //This socket is now waiting to be worked
                if let Ok(mut waiting) = self.poll.1.lock() {
                    (0..cnt).for_each(|_|{
                        waiting.push(socket_id)
                    });
                };
                self.poll.0.notify_all();
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
                connection.data_buffer.read().map(Packet::Data)
            }
            None => None,
        }
    }

    pub fn loss(&mut self, socket_id: u16, loss: SequenceRange) {
        if let Some(connection) = self.connections.get_mut(&socket_id) {
            if loss.start < loss.stop && loss.stop < connection.data_buffer.last_seq() {
                connection.loss_buffer.insert(loss);
            }
        }
    }
    pub fn out_of_sequence(&mut self, socket_id: u16, ack_no: SequenceNumber) -> bool {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                connection.data_buffer.last_seq() < ack_no
            },
            None => false,
        }
    }
    pub fn out_of_sequence_square(&mut self, socket_id: u16, ack_no: SequenceNumber) -> bool {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => connection.data_buffer.last_ack() < ack_no,
            None => false,
        }
    }
    pub fn remove_confirmed(&mut self, socket_id: u16, ack_no: SequenceNumber) {
        if let Some(connection) = self.connections.get_mut(&socket_id) {
            connection.loss_buffer.remove_confirmed(ack_no);
        }
    }
    pub fn ack(&mut self, socket_id: u16, ack_no: SequenceNumber) -> bool {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => connection.data_buffer.ack(ack_no),
            None => false,
        }
    }
    pub fn ack_square(&mut self, socket_id: u16, ack_no: SequenceNumber) {
        if let Some(connection) = self.connections.get_mut(&socket_id) {
            connection.data_buffer.ack_square(ack_no);
        }
    }

    //Can't lock here, need to keep this open
    pub fn poll(&self) -> Arc<(Condvar, Arc<Mutex<Vec<u16>>>)> {
        self.poll.clone()
    }
    pub fn next_time(&mut self) -> Option<(u16, Duration)> {
        //condvar wait for something in the queue
        let result = self
            .connections
            .iter()
            .fold(None, |acc, (c_sock, c_conn)| match acc {
                Some((_, acc_delay)) => match c_conn.updates.peek() {
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
                },
                None => match c_conn.updates.peek() {
                    Some(time) => match time.duration_since(SystemTime::now()) {
                        Ok(delay) => Some((*c_sock, delay)),
                        Err(_) => Some((*c_sock, Duration::ZERO)),
                    },
                    None => None,
                },
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
            Some(connection) => {
                //Remove the first count of this socket from the queue
                if let Ok(mut waiting) = self.poll.1.lock() {
                    if let Some(pos) = waiting.iter().position(|&x| x == socket_id) {
                        waiting.remove(pos);
                    }
                };
                match connection.updates.pop() {
                    Some(time) => match time.duration_since(SystemTime::now()) {
                        Ok(delay) => Some(delay),
                        Err(_) => None,
                    },
                    None => None,
                }
            }
            None => None,
        }
    }
    pub fn last_seq(&self, socket_id: u16) -> Option<SequenceNumber> {
        self.connections.get(&socket_id).map(|connection| connection.data_buffer.last_seq())
    }

    pub fn keep_alive(&mut self,socket_id: u16)->bool{
        match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                match connection.data_buffer.keep_alive(){
                    Some(seq_range)=>{
                        connection.loss_buffer.insert(seq_range);
                        if let Ok(mut ws) = self.poll.1.lock(){
                            ws.push(socket_id);
                        }
                        connection.updates.push(SystemTime::now());
                        self.poll.0.notify_all();
                        false
                    },
                    None=>true
                }
            },
            None => false,
        }    }
}
