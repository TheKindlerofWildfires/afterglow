use std::{
    sync::{Arc, RwLock}, time::Duration}
;

use crate::{
    core::channel::NeonChannel,
    packet::data::DataPacket,
    utils::{MessageNumber, SequenceNumber, SequenceRange},
};

use super::recv_list::RecvList;

pub struct RecvQueue {
    list: Arc<RwLock<RecvList>>,
    channel: Arc<RwLock<NeonChannel>>,
    closing: Arc<RwLock<bool>>,
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
    pub fn new(channel: Arc<RwLock<NeonChannel>>, closing: Arc<RwLock<bool>>) -> Self {
        let list = Arc::new(RwLock::new(RecvList::new()));
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
    pub fn drop_msg(&self, socket_id: u16, msg_no: MessageNumber, range: SequenceRange) {
        match self.list.write() {
            Ok(mut binding) => binding.drop_msg(socket_id, msg_no, range),
            Err(_) => self.close(),
        }
    }

    pub fn process_data(&mut self, packet: DataPacket,mss: u16) -> Vec<SequenceRange> {
        match self.list.write() {
            Ok(mut binding) => {
                let socket_id = packet.dst_socket_id;
                //add the data
                binding.add_data(packet);
                //trigger an up to date loss list
                binding.report_loss(socket_id,mss)

            },
            Err(_) => {
                self.close();
                vec![]
            },
        }
    }

    pub fn update_delay(&self, socket_id: u16, factor: f64) {
        match self.list.write() {
            Ok(mut binding) => binding.update_delay(socket_id, factor),
            Err(_) => self.close(),
        }        
    }
    pub fn next_ack(&self, socket_id: u16) -> Option<SequenceNumber> {
        match self.list.write() {
            Ok(mut binding) => binding.next_ack(socket_id),
            Err(_) => None,
        }  
    }
    pub fn inc_ack(&self, socket_id: u16) -> Option<SequenceNumber> {
        match self.list.write() {
            Ok(mut binding) => binding.inc_ack(socket_id),
            Err(_) => None,
        }  
    }

    pub fn time_data(&self, socket_id: u16)->Option<(u16,Duration,Duration)>{
        match self.list.read() {
            Ok(binding) => {
                Some((binding.buffer_size(socket_id), binding.recv_speed(socket_id), binding.bandwidth(socket_id)))                   
            },
            Err(_) => None,
        }  
    }

    pub fn remove(&mut self, socket_id: u16){
        match self.list.write(){
            Ok(mut binding) => {
                binding.remove_connection(socket_id)
            },
            Err(_) => {},
        }
    }
    pub fn read_data(&self, socket_id: u16) -> Option<Vec<u8>> {
        match self.list.write(){
            Ok(mut binding) => {
                binding.pop_data(socket_id)
            },
            Err(_) => None,
        }
    }
}
