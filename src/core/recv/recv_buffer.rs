use std::{cmp::min, collections::HashMap, time::{Duration, SystemTime}};

use crate::{
    congestion::CongestionController, core::SYN_INTERVAL, packet::data::{DataPacket, DataPacketType}, utils::{MessageNumber, SequenceNumber, SequenceRange}, window::ack_window::Window
};

//This structure only tracks data messages and outputs fully formed packets
pub struct RecvBuffer {
    last_msg: MessageNumber, //the last msg popped
    last_seq: SequenceNumber, //the last seq processed
    last_ack: SequenceNumber, //the last ack sent
    last_ack_square: SequenceNumber, //the last ack square sent
    last_ack_time: SystemTime,
    last_ack_square_time: SystemTime,
    next_ack_time: SystemTime,
    ack_window: Window,
    congestion: CongestionController,
    blocks: HashMap<MessageNumber, RecvBlock>,
}

impl RecvBuffer {
    pub fn new(self_isn: SequenceNumber, partner_isn: SequenceNumber) -> Self {
        dbg!(self_isn,partner_isn);
        let last_msg = MessageNumber::ZERO;
        let blocks = HashMap::new();
        let last_seq = self_isn;
        let last_ack = self_isn;
        let last_ack_square = last_ack;
        let last_ack_time = SystemTime::now();
        let last_ack_square_time = SystemTime::now();
        let next_ack_time = SystemTime::now()+SYN_INTERVAL;
        let congestion = CongestionController::new();
        let ack_window = Window::new(Duration::from_millis(2000));
        Self {
            last_msg,
            last_seq,
            blocks,
            last_ack,
            last_ack_square,
            last_ack_time,
            last_ack_square_time,
            next_ack_time,
            ack_window,
            congestion
        }
    }

    pub fn add(&mut self, packet: DataPacket) -> Option<SequenceRange> {
        let msg_no = packet.msg_no;
        //check for dropped packets
        let mut start = self.last_seq;
        start.inc();
        let skip_range = if packet.seq_no > start {
            //someone dropped a packet
            let mut stop = packet.seq_no;
            stop.dec();
            Some(SequenceRange { start, stop })
        } else {
            None
        };
        //update sequence numbers
        if packet.seq_no > self.last_seq {
            self.last_seq = packet.seq_no;
        }
        //put the data into the list
        match self.blocks.get_mut(&msg_no) {
            //Append to existing messages
            Some(recv_block) => {
                recv_block.data.push(packet);
                recv_block.update_state();
            }
            //Create a new message
            None => {
                let state = BlockState::Partial;
                let _stamp = packet.stamp;
                let data = vec![packet];
                let mut recv_block = RecvBlock { _stamp, data, state };
                recv_block.update_state();
                self.blocks.insert(msg_no, recv_block);
            }
        }
        skip_range
    }
    pub fn pop(&mut self) -> Option<Vec<u8>> {
        //Find all blocks for this socket

        //Find all complete blocks
        let mut complete_blocks = self
            .blocks
            .iter_mut()
            .filter(|(_, block)| block.state == BlockState::Complete)
            .collect::<Vec<_>>();
        //Sort the block by message number
        complete_blocks.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        let out = match complete_blocks.first_mut() {
            Some((msg_no, block)) => {
                //check that this is the next message in the sequence

                match **msg_no == self.last_msg {
                    true => {
                        let bytes = block.to_bytes();
                        self.last_msg.inc();
                        Some((**msg_no, bytes))
                    }
                    false => None,
                }
            }
            None => None,
        };

        match out {
            Some((msg_no, bytes)) => {
                self.blocks.remove(&msg_no);
                Some(bytes)
            }
            None => None,
        }
    }

    pub fn drop_msg(&mut self, msg_no: MessageNumber, range: SequenceRange) {
        self.blocks.remove(&msg_no);
        let mut prev_msg = msg_no;
        prev_msg.dec();
        if self.last_msg == prev_msg {
            self.last_msg = msg_no
        }
        if range.start < self.last_seq && range.stop > self.last_seq {
            self.last_seq = range.stop
        }
    }

    pub fn size(&self) -> usize {
        self.blocks.len()
    }
    pub fn last_seq(&self)->SequenceNumber{
        self.last_seq
    }
    pub fn sent_ack(&mut self,ack_no:SequenceNumber){
        if ack_no>self.last_ack{
            self.last_ack = ack_no
        }
        self.ack_window.store(ack_no, ack_no)
    }
    pub fn next_ack(&mut self)->SequenceNumber{
        self.last_seq
    }
    pub fn ack_square(&mut self, ack_no:SequenceNumber){
        self.last_ack_square_time = SystemTime::now();
        if ack_no>self.last_ack_square{
            self.last_ack_square =  ack_no;
        }
        let mut _discard = SequenceNumber::new(0);
        let rtt = self.ack_window.acknowledge(ack_no, &mut _discard);
        self.congestion.update_rtt(rtt.as_millis() as u16);
    }
    pub fn should_ack(&mut self,proposed_ack: SequenceNumber)->Option<(SequenceNumber,SequenceNumber)>{
        let should_ack = match self.next_ack_time.elapsed() {
            Ok(elapse) => {
                true
            }
            Err(err) => {
                self.congestion.should_ack()
            }
        };
        //If the time is wrong don't ack
        if !should_ack{
            return None;
        }
        //if we've confirmed this ack don't send it
        if proposed_ack==self.last_ack_square{
            return None;
        }


        //If this is a new ack or the last ack timed out
        if proposed_ack>self.last_ack{
            self.last_ack = proposed_ack;
        }else if proposed_ack==self.last_ack{
            let ack_timeout = match self.last_ack_time.elapsed(){
                Ok(dur)=>dur<self.congestion.long_poll(),
                Err(_)=>false 
            };
            if !ack_timeout{
                return None
            }
        }else{
            return None
        }
        if self.last_ack>self.last_ack_square{
            self.next_ack_time = SystemTime::now()+self.congestion.next_ack();
            return Some((proposed_ack,self.last_seq))
        }
        None
    }
    pub fn loss(&mut self, loss_ranges: Vec<SequenceRange>){
        let loss_start = loss_ranges.iter().fold(SequenceNumber::MAX_SEQ_NO, |acc, range|{
            min(range.start,acc)
        });
        self.congestion.on_loss(loss_start);
    }
    pub fn delay(&self)->Duration{
        self.congestion.next_time()
    }
    pub fn on_pkt(&mut self){
        self.congestion.inc_pkt_cnt();
    }
    pub fn rtt(&self)->(Duration,Duration){
        self.congestion.rtt()
    }
    pub fn update_rtt(&mut self, rtt: u16){
        self.congestion.update_rtt(rtt)
    }
    pub fn update_recv_rate(&mut self, rate: u16){
        self.congestion.update_recv_rate(rate)
    }
    pub fn update_bandwidth(&mut self, bw: u16){
        self.congestion.update_bandwidth(bw)
    }
    pub fn on_ack(&mut self, ack_no:SequenceNumber){
        self.congestion.on_ack(ack_no);
    }
}

#[derive(Clone, Debug)]
pub struct RecvBlock {
    _stamp: SystemTime,
    data: Vec<DataPacket>,
    state: BlockState,
}

impl RecvBlock {
    pub fn update_state(&mut self) {
        //if this is solo it's complete
        self.state = if self.data[0].element == DataPacketType::Solo {
            BlockState::Complete
        } else {
            //order the data by seq_no
            self.data.sort_unstable_by(|a, b| a.seq_no.cmp(&b.seq_no));
            //easy check to make sure first and last are here
            if self.data[0].element == DataPacketType::First
                && self.data[self.data.len() - 1].element == DataPacketType::Last
            {
                //Make sure the sequence is fully there
                let mut first_seq = self.data[0].seq_no;
                first_seq.dec();
                let (valid, _) = self.data.iter().fold((true, first_seq), |acc, pkt| {
                    let mut next_seq = acc.1;
                    next_seq.inc();
                    if next_seq == pkt.seq_no {
                        (acc.0 & true, pkt.seq_no)
                    } else {
                        (false, pkt.seq_no)
                    }
                });
                if valid {
                    BlockState::Complete
                } else {
                    BlockState::Partial
                }
            } else {
                BlockState::Partial
            }
        }
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        //Assume sorted in update state
        let raw_bytes = self.data.iter().fold(Vec::new(), |mut acc, pkt| {
            acc.extend_from_slice(&pkt.data);
            acc
        });
        DataPacket::decompress(&raw_bytes)
    }

}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockState {
    Complete,
    Partial,
}
