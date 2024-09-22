use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

use crate::{
    core::SYN_INTERVAL,
    packet::data::{DataPacket, DataPacketType},
    utils::{MessageNumber, SequenceNumber, SequenceRange},
};

#[derive(Debug)]
pub struct SendBuffer {
    last_msg: MessageNumber, //the last msg no created
    last_seq: SequenceNumber, //the last seq no created
    last_ack: SequenceNumber, // the last ack recv
    last_ack_square: SequenceNumber, //the last ack square recv
    last_ack_time: SystemTime,
    last_ack_square_time: SystemTime,
    blocks: HashMap<SequenceNumber, SendBlock>,
    drops: HashMap<MessageNumber, Vec<SequenceNumber>>,
}
impl SendBuffer {
    pub fn new(self_isn: SequenceNumber, partner_isn: SequenceNumber) -> Self {
        let last_msg = MessageNumber::ZERO;
        let blocks = HashMap::new();
        let drops = HashMap::new();
        let last_seq = self_isn;
        let last_ack = self_isn;
        let last_ack_square = last_ack;
        let last_ack_time = SystemTime::now();
        let last_ack_square_time = SystemTime::now();
        Self {
            last_seq,
            last_msg,
            last_ack,
            last_ack_square,
            last_ack_time,
            last_ack_square_time,
            blocks,
            drops,
        }
    }
    fn create_packets(
        &mut self,
        data: &[u8],
        mss: u16,
        order: bool,
        partner_id: u16,
    ) -> Vec<DataPacket> {
        let data = DataPacket::compress(data);

        //split up the data into chunks
        let count = data.chunks(mss as usize).count();
        if count == 0 {
            return vec![];
        }
        let stamp = SystemTime::now();

        if count == 1 {
            self.last_seq.inc();

            let packet = DataPacket::new(
                self.last_seq,
                self.last_msg,
                DataPacketType::Solo,
                order,
                stamp,
                partner_id,
                data.to_vec(),
            );
            self.last_msg.inc();

            return vec![packet];
        }
        let out = data
            .chunks(mss as usize)
            .enumerate()
            .map(|(i, chunk)| {
                self.last_seq.inc();

                let element = if i == 0 {
                    DataPacketType::First
                } else if i == count - 1 {
                    DataPacketType::Last
                } else {
                    DataPacketType::Middle
                };
                

                DataPacket::new(
                    self.last_seq,
                    self.last_msg,
                    element,
                    order,
                    stamp,
                    partner_id,
                    chunk.to_vec(),
                )
            })
            .collect::<Vec<_>>();
        self.last_msg.inc();
        out
    }

    pub fn add(
        &mut self,
        data: &[u8],
        ttl: Duration,
        order: bool,
        partner_id: u16,
        mss: u16,
    ) -> usize {
        let packets = self.create_packets(data, mss, order, partner_id);
        let len = packets.len();
        packets.into_iter().for_each(|packet| {
            let seq_no = packet.seq_no;
            let timeout = SystemTime::now() + ttl;
            let send_block = SendBlock {
                packet,
                timeout,
                state: BlockState::Fresh,
            };
            self.blocks.insert(seq_no, send_block);
        });
        len
    }

    fn manage_drops(&mut self) {
        let (keeps, drops): (Vec<_>, Vec<_>) =
            self.blocks
                .clone()
                .into_iter()
                .partition(|(_, block)| block.timeout.elapsed().is_err());
        drops.into_iter().for_each(|(seq_no, block)| {
            match self.drops.get_mut(&block.packet.msg_no) {
                Some(dropped_msg) => {
                    dropped_msg.push(seq_no);
                }
                None => {
                    self.drops.insert(block.packet.msg_no, vec![seq_no]);
                }
            }
        });
        self.blocks.clear();
        keeps.into_iter().for_each(|(seq_no, block)| {
            self.blocks.insert(seq_no, block);
        });
    }

    pub fn read(&mut self) -> Option<DataPacket> {
        self.manage_drops();
        //find the first unsent element, mark it as seen
        let mut fresh_blocks = self
            .blocks
            .iter_mut()
            .filter(|(_, block)| block.state == BlockState::Fresh)
            .collect::<Vec<_>>();
        fresh_blocks.sort_by(|a, b| a.0.cmp(b.0));
        match fresh_blocks.first_mut() {
            Some((_, block)) => {
                let out = block.packet.clone();
                block.state = BlockState::Read;
                Some(out)
            }
            None => None,
        }
    }

    /* */
    pub fn read_recall(&mut self, msg_no: MessageNumber) -> Option<DataPacket> {
        self.manage_drops();
        self
            .blocks
            .iter()
            .find(|(_, block)| block.packet.msg_no == msg_no).map(|(_, block)| block.packet.clone())
    }
    pub fn search(&mut self, range: SequenceRange) -> Option<DataPacket> {
        dbg!(&self.blocks.len());
        
        let candidates = self
            .blocks
            .iter()
            .filter(|(seq_no, _)| range.contains(**seq_no)).map(|(seq_no, block)| (*seq_no,block.packet.clone())).collect::<Vec<_>>();
        let first = candidates.iter().fold((SequenceNumber::MAX_SEQ_NO, None), |acc, c|{
            if c.0<acc.0{
                (c.0, Some(c.1.clone()))
            }else{
                acc
            }
        });
        first.1
    }
    pub fn ack(&mut self, ack_no: SequenceNumber) -> bool {
        //Remove blocks and drops from before this ack number->problem because ack is wrong (TODO)
        self.blocks
            .retain(|seq_no, block| *seq_no > ack_no || block.state == BlockState::Fresh);
        self.drops
            .retain(|_, seqs| seqs.iter().any(|seq| *seq > ack_no));
        match self.last_ack_time.elapsed() {
            Ok(time) => {
                if time > SYN_INTERVAL || ack_no == self.last_ack {
                    self.last_ack = ack_no;
                    self.last_ack_time = SystemTime::now();
                    true
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    }

    pub fn ack_square(&mut self, ack_no: SequenceNumber) {
        self.last_ack_square = ack_no;
        self.last_ack_square_time = SystemTime::now();
    }

    //To clean up when ready
    pub fn drop(&mut self) -> HashMap<MessageNumber, Vec<SequenceNumber>> {
        let out = self.drops.clone();
        self.drops.clear();
        out
    }

    pub fn size(&self) -> usize {
        self.blocks.len()
    }
    pub fn last_seq(&self) -> SequenceNumber {
        self.last_seq
    }
    pub fn last_ack(&self) -> SequenceNumber {
        self.last_ack
    }
    pub fn keep_alive(&self) ->Option<SequenceRange>{
        if self.last_seq != self.last_ack {
            Some(SequenceRange{
                start: self.last_ack,
                stop: self.last_seq,
            })
        }else{
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct SendBlock {
    packet: DataPacket,
    timeout: SystemTime,
    state: BlockState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockState {
    Fresh,
    Read,
}
