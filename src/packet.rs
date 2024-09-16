pub mod control;
pub mod data;


use control::ControlPacket;
use data::DataPacket;

use crate::serial::Serial;

pub const HEADER_SIZE:usize = 8;

#[derive(Clone, Debug)]
pub enum Packet {
    Control(ControlPacket),
    Data(DataPacket),
}

impl Packet {
    pub fn socket_id(&self) -> u16 {
        match self {
            Packet::Control(packet) => packet.dst_socket_id,
            Packet::Data(packet) => packet.dst_socket_id,
        }
    }
}

impl Serial for Packet {
    fn serialize(&self) -> Vec<u8> {
        let bytes = match self {
            Packet::Control(packet) => {
                let mut bytes = packet.serialize();
                bytes[0] |= 0x80;
                bytes
            }
            Packet::Data(packet) => packet.serialize(),
        };
        bytes
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        match bytes[0] & 0x80 {
            0x80 => Packet::Control(ControlPacket::deserialize(
                bytes,
                start,
            )),
            0x00 => Packet::Data(DataPacket::deserialize(bytes, start)),
            _ => unreachable!(),
        }
    }
}
