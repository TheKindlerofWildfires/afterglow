use crate::{
    huffman::{Bitter, CodeBuilder, Tree},
    serial::Serial,
    utils::{MessageNumber, SequenceNumber},
};
use std::{collections::HashMap, fmt::Debug, time::SystemTime};

#[derive(Clone)]
pub struct DataPacket {
    pub seq_no: SequenceNumber,
    pub msg_no: MessageNumber,
    pub element: DataPacketType,
    pub order: bool,
    pub stamp: SystemTime,
    pub dst_socket_id: u16,
    pub data: Vec<u8>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DataPacketType {
    Solo,
    First,
    Last,
    Middle,
}

impl DataPacket {
    pub fn new(
        seq_no: SequenceNumber,
        msg_no: MessageNumber,
        element: DataPacketType,
        order: bool,
        stamp: SystemTime,
        dst_socket_id: u16,
        data: Vec<u8>,
    ) -> Self {
        Self {
            seq_no,
            msg_no,
            element,
            order,
            stamp,
            dst_socket_id,
            data,
        }
    }
    pub fn compress(bytes: &[u8]) -> Vec<u8> {
        //Huffman encode all packets
        let mut weights = HashMap::new();
        for &byte in bytes {
            let counter = weights.entry(byte).or_insert(0u16);
            *counter += 1;
        }
        let (book, tree) = CodeBuilder::from_iter(weights).finish();
        let mut bitter = tree.bitter_serial();
        for byte in bytes {
            book.encode(&mut bitter, byte)
                .expect("Invariant failed in huffman");
        }
        bitter.serialize()
    }

    pub fn decompress(bytes: &[u8]) -> Vec<u8> {
        let mut bitter = Bitter::deserialize(bytes, &mut 0);
        let mut ptr = 0;
        let tree = Tree::bitter_deserial(&bitter, &mut ptr);
        bitter.forward(ptr);
        tree.unbounded_decoder(bitter).collect()
    }
}

impl Serial for DataPacket {
    fn serialize(&self) -> Vec<u8> {
        let mut bytes = self.seq_no.serialize();
        let mut msg_no = self.msg_no.serialize();
        match self.element {
            DataPacketType::Solo => msg_no[0] |= 0xc0,
            DataPacketType::First => msg_no[0] |= 0x80,
            DataPacketType::Last => msg_no[0] |= 0x40,
            DataPacketType::Middle => {}
        }
        match self.order {
            true => msg_no[0] |= 0x20,
            false => {}
        }
        bytes.extend_from_slice(&msg_no);
        bytes.extend_from_slice(&self.stamp.serialize());
        bytes.extend_from_slice(&self.dst_socket_id.serialize());
        bytes.extend_from_slice(&self.data);
        bytes
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let seq_no = SequenceNumber::deserialize(bytes, start);
        let control = bytes[*start];
        let msg_no = MessageNumber::deserialize(bytes, start);
        let element = match control & 0xc0 {
            0xc0 => DataPacketType::Solo,
            0x80 => DataPacketType::First,
            0x40 => DataPacketType::Last,
            _ => DataPacketType::Middle,
        };
        let order = matches!(control & 0x20, 0x20);
        let stamp = SystemTime::deserialize(bytes, start);
        let dst_socket_id = u16::deserialize(bytes, start);

        let data = bytes[*start..].to_vec();
        *start = bytes.len();
        Self {
            seq_no,
            msg_no,
            element,
            order,
            stamp,
            dst_socket_id,
            data,
        }
    }
}

impl Debug for DataPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataPacket")
            .field("seq_no", &self.seq_no)
            .field("msg_no", &self.msg_no)
            .field("element", &self.element)
            .field("order", &self.order)
            .field("stamp", &self.stamp)
            .field("dst_socket_id", &self.dst_socket_id)
            .field("data.len()", &self.data.len())
            .finish()
    }
}
