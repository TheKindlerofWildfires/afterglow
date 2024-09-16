use std::{collections::HashMap, time::SystemTime};

use crate::{
    packet::data::{DataPacket, DataPacketType},
    utils::{MessageNumber, SequenceNumber, SequenceRange},
};

//This structure only tracks data messages and outputs fully formed packets
pub struct RecvBuffer {
    last_seq: SequenceNumber,
    last_ack: SequenceNumber,
    last_ack_square: SequenceNumber,
    last_ack_time: SystemTime,
    last_ack_square_time: SystemTime,
    last_msg: MessageNumber,
    blocks: HashMap<MessageNumber, RecvBlock>,
}

impl RecvBuffer {
    pub fn new(last_seq: SequenceNumber) -> Self {
        let last_msg = MessageNumber::ZERO;
        let blocks = HashMap::new();
        let mut last_ack = last_seq;
        last_ack.dec();
        let last_ack_square = last_ack;
        let last_ack_time = SystemTime::now();
        let last_ack_square_time = SystemTime::now();
        Self {
            last_msg,
            last_seq,
            blocks,
            last_ack,
            last_ack_square,
            last_ack_time,
            last_ack_square_time
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
                let stamp = packet.stamp;
                let data = vec![packet];
                let mut recv_block = RecvBlock { stamp, data, state };
                recv_block.update_state();
                if msg_no < self.last_msg {
                    self.last_msg = msg_no;
                }
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
    pub fn inc_ack(&mut self)->SequenceNumber{
        self.last_ack.inc();
        self.last_ack
    }
}

#[derive(Clone, Debug)]
pub struct RecvBlock {
    stamp: SystemTime,
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
