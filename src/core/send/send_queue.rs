use std::{
    io::{Error, ErrorKind}, net::SocketAddr, sync::{Arc, Condvar, Mutex, RwLock}, time::Duration
};

use crate::{
    core::channel::NeonChannel,
    packet::Packet,
    utils::{SequenceNumber, SequenceRange},
};

use super::send_list::SendList;

pub struct SendQueue {
    list: Arc<RwLock<SendList>>,
}

/*
    This takes outbound data from streams and puts them into the channel queue
    It also works the channel queue and determines when things need to get sent
    It also exposes a place for streams to take packets from (via core)
    It's also responsible for exposing the necessary elements of control functionality to core
*/

//I think this should have a 'neon in' and 'udp out' thread and reverse for recv if they can be made to work together
impl SendQueue {
    pub fn new() -> Self {
        let list = Arc::new(RwLock::new(SendList::new()));
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

    pub fn push_data(
        &mut self,
        socket_id: u16,
        data: &[u8],
        ttl: Duration,
        order: bool,
        partner_id: u16,
        mss: u16,
    ) -> usize {
        match self.list.write() {
            Ok(mut binding) => binding.insert(socket_id, data, ttl, order, partner_id, mss),
            Err(_) => 0,
        }
    }
    pub fn send_packet(&self, channel: &NeonChannel, addr: SocketAddr, packet: Packet)->Result<usize, Error> {
        channel.send_to(addr, packet)
    }
    pub fn send_data(&mut self, channel: &NeonChannel, addr: SocketAddr, socket_id: u16)->Result<usize, Error> {
        let packet = match self.list.write() {
            Ok(mut binding) => binding.pop(socket_id),
            Err(_) => return Err(Error::new(ErrorKind::Interrupted, "Poisoned lock")),
        };
        match packet {
            Some(packet) => self.send_packet(channel, addr, packet),
            None => Ok(0)
        }
    }
    pub fn next_time(&self) -> Option<(u16, Duration)> {
        match self.list.write() {
            Ok(mut list) => list.next_time(),
            Err(_) => None,
        }
    }
    pub fn loss(&mut self, socket_id: u16, loss: SequenceRange) {
        match self.list.write() {
            Ok(mut list) => list.loss(socket_id, loss),
            Err(_) => {}
        }
    }
    pub fn ack(&mut self, socket_id: u16, ack_no: SequenceNumber) -> bool {
        match self.list.write() {
            Ok(mut list) => {
                if !list.out_of_sequence(socket_id, ack_no) {
                    list.remove_confirmed(socket_id, ack_no);
                    list.ack(socket_id, ack_no)
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    }
    pub fn ack_square(&mut self, socket_id: u16, ack_no: SequenceNumber) {
        match self.list.write() {
            Ok(mut list) => {
                if !list.out_of_sequence_square(socket_id, ack_no) {
                    list.ack_square(socket_id, ack_no);
                }
            }
            Err(_) => {}
        }
    }
    pub fn update(&mut self, socket_id: u16, cnt: usize, delay: Duration) {
        match self.list.write() {
            Ok(mut list) => list.update(socket_id, cnt,delay),
            Err(_) => {}
        }
    }

    pub fn remove(&mut self, socket_id: u16) {
        match self.list.write() {
            Ok(mut binding) => binding.remove_connection(socket_id),
            Err(_) => {}
        }
    }
    pub fn last_seq(&self, socket_id: u16)->Option<SequenceNumber>{
        match self.list.read() {
            Ok(binding) => binding.last_seq(socket_id),
            Err(_) => None
        }
    }
    pub fn poll(&self)->Option<Arc<(Condvar,Arc<Mutex<Vec<u16>>>)>>{
        match self.list.read() {
            Ok(binding) => Some(binding.poll()),
            Err(_) => None
        }
    }
    pub fn keep_alive(&self,socket_id: u16)->bool{
        match self.list.write(){
            Ok(mut binding)=>{
                binding.keep_alive(socket_id)
            },
            Err(_)=>false
        }
    }
}
