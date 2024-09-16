use std::{
    net::SocketAddr, sync::{Arc, RwLock}, time::{Duration, SystemTime}
};

use crate::{core::channel::NeonChannel, packet::Packet, utils::{SequenceNumber, SequenceRange}};

use super::send_list::SendList;

pub struct SendQueue {
    list: Arc<RwLock<SendList>>,
    channel: Arc<RwLock<NeonChannel>>,
    closing: Arc<RwLock<bool>>,
}

/*
    This takes outbound data from streams and puts them into the channel queue
    It also works the channel queue and determines when things need to get sent
    It also exposes a place for streams to take packets from (via core)
    It's also responsible for exposing the necessary elements of control functionality to core
*/

//I think this should have a 'neon in' and 'udp out' thread and reverse for recv if they can be made to work together
impl SendQueue {
    pub fn new(channel: Arc<RwLock<NeonChannel>>, closing: Arc<RwLock<bool>>) -> Self {
        let list = Arc::new(RwLock::new(SendList::new()));
        Self {
            list,
            closing,
            channel,
        }
    }
    pub fn close(&self) {
        match self.closing.write() {
            Ok(mut closing) => *closing = true,
            Err(_) => {}
        };
    }
    pub fn register_connection(&self, socket_id: u16, isn: SequenceNumber) {
        match self.list.write() {
            Ok(mut binding) => binding.register_connection(socket_id, isn),
            Err(_) => self.close(),
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
    ) -> usize{
        match self.list.write() {
            Ok(mut binding) => binding.insert(socket_id, data, ttl, order, partner_id, mss),
            Err(_) => 0,
        }
    }
    pub fn send_packet(&self, addr: SocketAddr, packet: Packet) {
        match self.channel.read(){
            Ok(binding)=>{
                match binding.send_to(addr, packet) {
                    Ok(_) => {}
                    Err(_) => self.close()
                }
            },
            Err(_)=>self.close()
        }       
    }
    pub fn send_data(&mut self, addr:SocketAddr, socket_id: u16){
        let packet = match self.list.write(){
            Ok(mut binding) => {
                binding.pop(socket_id)

            },
            Err(_) => return,
        };
        match packet{
            Some(packet) =>  self.send_packet(addr, packet),
            None => {},
        }
       ;
    }
    pub fn next_time(&self)->Option<(u16, Duration)>{
        match self.list.write(){
            Ok(mut list)=>{
                list.next_time()
            },
            Err(_)=>None
        }
    }
    pub fn loss(&mut self, socket_id: u16, loss: SequenceRange) {
        match self.list.write(){
            Ok(mut list) => list.loss(socket_id,loss),
            Err(_) => {},
        }
    }
    pub fn ack(&mut self, socket_id: u16, ack_no: SequenceNumber) {
        match self.list.write(){
            Ok(mut list) => {
                if !list.out_of_sequence(socket_id,ack_no){
                    list.remove_confirmed(socket_id, ack_no);
                }
            }
            Err(_) => {},
        }
    }
    pub fn update(&mut self, socket_id: u16, schedule: SystemTime){
        match self.list.write(){
            Ok(mut list) => list.update(socket_id,schedule),
            Err(_) => {},
        }
    }

    /* 
    pub fn drops(&self) -> Vec<SendBlock> {
        let mut list_lock = self.list.write().unwrap();
        list_lock.drops()
    }*/
    pub fn remove(&mut self, socket_id: u16){
        match self.list.write(){
            Ok(mut binding) => {
                binding.remove_connection(socket_id)
            },
            Err(_) => {},
        }
    }
}
