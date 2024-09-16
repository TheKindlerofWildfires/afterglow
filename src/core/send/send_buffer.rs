use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

use crate::{
    packet::data::{DataPacket, DataPacketType},
    utils::{MessageNumber, SequenceNumber, SequenceRange},
};

#[derive(Debug)]
pub struct SendBuffer {
    last_msg: MessageNumber,
    last_seq: SequenceNumber,
    blocks: HashMap<SequenceNumber, SendBlock>,
    drops: HashMap<MessageNumber, Vec<SequenceNumber>>,
}
impl SendBuffer {
    pub fn new(last_seq: SequenceNumber) -> Self {
        let last_msg = MessageNumber::ZERO;
        let blocks = HashMap::new();
        let drops = HashMap::new();
        Self {
            last_seq,
            last_msg,
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
            let packet = DataPacket::new(
                self.last_seq,
                self.last_msg,
                DataPacketType::Solo,
                order,
                stamp,
                partner_id,
                data.to_vec(),
            );
            self.last_seq.inc();
            self.last_msg.inc();

            return vec![packet];
        }
        let out = data
            .chunks(mss as usize)
            .enumerate()
            .map(|(i, chunk)| {
                let element = if i == 0 {
                    DataPacketType::First
                } else if i == count - 1 {
                    DataPacketType::Last
                } else {
                    DataPacketType::Middle
                };
                let packet = DataPacket::new(
                    self.last_seq,
                    self.last_msg,
                    element,
                    order,
                    stamp,
                    partner_id,
                    chunk.to_vec(),
                );

                self.last_seq.inc();
                packet
            })
            .collect::<Vec<_>>();
        self.last_msg.inc();
        out
    }

    pub fn add(&mut self, data: &[u8], ttl: Duration, order: bool, partner_id: u16, mss: u16) -> usize{
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
                .partition(|(_, block)| match block.timeout.elapsed() {
                    Ok(_) => false,
                    Err(_) => true,
                });
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
        match self
            .blocks
            .iter()
            .find(|(_, block)| block.packet.msg_no == msg_no)
        {
            Some((_, block)) => Some(block.packet.clone()),
            None => None,
        }
    }
    pub fn search(&mut self, range: SequenceRange) -> Option<DataPacket> {
        match self.blocks.iter().find(|(seq_no,_)|{
            range.contains(**seq_no)
        }){
            Some((_,block)) => Some(block.packet.clone()),
            None => None,
        }
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
    pub fn last_seq(&self)->SequenceNumber{
        self.last_seq
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
