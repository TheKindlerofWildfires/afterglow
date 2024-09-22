use std::{net::SocketAddr, time::SystemTime};

use crate::{
    serial::Serial,
    utils::{MessageNumber, SequenceNumber, SequenceRange},
};
use ack::Ack;
use ack_square::AckSquare;
use congestion::Congestion;
use custom::Custom;
use discover::Discover;
use drop::Drop;
use err::Err;
use handshake::{Handshake, ReqType, FLOW_CONTROL};
use keep_alive::KeepAlive;
use loss::Loss;
use shutdown::Shutdown;

pub mod ack;
pub mod ack_square;
pub mod congestion;
pub mod custom;
pub mod discover;
pub mod drop;
pub mod err;
pub mod handshake;
pub mod keep_alive;
pub mod loss;
pub mod shutdown;


#[derive(Clone, Debug)]

pub struct ControlPacket {
    pub control_type: ControlType,
    pub meta: ControlMeta, //this might need a enum
    pub stamp: SystemTime,
    pub dst_socket_id: u16, //dst socket
    pub info: ControlPacketInfo,
}

#[derive(Clone, Debug)]

pub enum ControlMeta {
    Seq(SequenceNumber),
    Message(MessageNumber),
    Other(u16),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ControlType {
    Handshake,
    KeepAlive,
    Ack,
    Loss,
    Congestion,
    Shutdown,
    AckSquare,
    Drop,
    Err,
    Discover,
    Custom,
}
#[derive(Clone, Debug)]
pub enum ControlPacketInfo {
    Handshake(Handshake),
    KeepAlive(KeepAlive),
    Ack(Ack),
    Loss(Loss),
    Congestion(Congestion),
    Shutdown(Shutdown),
    AckSquare(AckSquare),
    Drop(Drop),
    Err(Err),
    Discover(Discover),
    Custom(Custom),
}
impl ControlPacket {
    pub fn handshake(
        dst_socket_id: u16,
        req_type: ReqType,
        mss: u16,
        src_socket_id: u16,
        in_addr: SocketAddr,
    ) -> Self {
        let control_type = ControlType::Handshake;
        let meta = ControlMeta::Other(0);
        let stamp = SystemTime::now();
        let info = ControlPacketInfo::Handshake(Handshake::new(
            req_type,
            mss,
            FLOW_CONTROL,
            src_socket_id,
            in_addr,
        ));
        Self {
            control_type,
            meta,
            stamp,
            dst_socket_id,
            info,
        }
    }
    pub fn keep_alive(dst_socket_id: u16) -> Self {
        let control_type = ControlType::KeepAlive;
        let meta = ControlMeta::Other(0);
        let stamp = SystemTime::now();
        let info = ControlPacketInfo::KeepAlive(KeepAlive::new());
        Self {
            control_type,
            meta,
            stamp,
            dst_socket_id,
            info,
        }
    }

    pub fn ack(
        dst_socket_id: u16,
        ack_no: SequenceNumber,
        seq_no: SequenceNumber,
        rtt: u16,
        rrt_variance: u16,
        buffer_size: u16,
        window: u16,
        bandwidth: u16,
    ) -> Self {
        let control_type = ControlType::Ack;
        let meta = ControlMeta::Seq(ack_no);
        let stamp = SystemTime::now();
        let info =
            ControlPacketInfo::Ack(Ack::new(seq_no, rtt, rrt_variance, buffer_size, window, bandwidth));
        Self {
            control_type,
            meta,
            stamp,
            dst_socket_id,
            info,
        }
    }
    pub fn loss(dst_socket_id: u16, ranges: Vec<SequenceRange>) -> Self {
        let control_type = ControlType::Loss;
        let meta = ControlMeta::Seq(SequenceNumber(0));
        let stamp = SystemTime::now();
        let info = ControlPacketInfo::Loss(Loss::new(ranges));
        Self {
            control_type,
            meta,
            stamp,
            dst_socket_id,
            info,
        }
    }
    pub fn congestion(dst_socket_id: u16, factor: u16) -> Self {
        let control_type = ControlType::Congestion;
        let meta = ControlMeta::Other(factor);
        let stamp = SystemTime::now();
        let info = ControlPacketInfo::Congestion(Congestion::new());
        Self {
            control_type,
            meta,
            stamp,
            dst_socket_id,
            info,
        }
    }

    pub fn shutdown(dst_socket_id: u16, code: u16) -> Self {
        let control_type = ControlType::Shutdown;
        let meta = ControlMeta::Other(code);
        let stamp = SystemTime::now();
        let info = ControlPacketInfo::Shutdown(Shutdown::new());
        Self {
            control_type,
            meta,
            stamp,
            dst_socket_id,
            info,
        }
    }
    pub fn ack_square(dst_socket_id: u16, ack_no: SequenceNumber) -> Self {
        let control_type = ControlType::AckSquare;
        let meta = ControlMeta::Seq(ack_no);
        let stamp = SystemTime::now();
        let info = ControlPacketInfo::AckSquare(AckSquare::new());
        Self {
            control_type,
            meta,
            stamp,
            dst_socket_id,
            info,
        }
    }

    pub fn drop(dst_socket_id: u16, msg_no: MessageNumber, range: SequenceRange) -> Self {
        let control_type = ControlType::Drop;
        let meta = ControlMeta::Message(msg_no);
        let stamp = SystemTime::now();
        let info = ControlPacketInfo::Drop(Drop::new(range));
        Self {
            control_type,
            meta,
            stamp,
            dst_socket_id,
            info,
        }
    }
    pub fn error(dst_socket_id: u16, code: u16) -> Self {
        let control_type = ControlType::Err;
        let meta = ControlMeta::Other(code);
        let stamp = SystemTime::now();
        let info = ControlPacketInfo::Err(Err::new());
        Self {
            control_type,
            meta,
            stamp,
            dst_socket_id,
            info,
        }
    }
    pub fn discovery(dst_socket_id: u16, in_mss: u16, out_mss: u16,req_type: ReqType) -> Self {
        let control_type = ControlType::Discover;
        let meta = ControlMeta::Other(in_mss);
        let stamp = SystemTime::now();
        let info = ControlPacketInfo::Discover(Discover::new(out_mss.into(),req_type));
        Self {
            control_type,
            meta,
            stamp,
            dst_socket_id,
            info,
        }
    }


}

impl Serial for ControlPacket {
    fn serialize(&self) -> Vec<u8> {
        let mut bytes = self.control_type.serialize();
        let meta = match self.meta {
            ControlMeta::Seq(ack) => ack.serialize(),
            ControlMeta::Message(msg) => msg.serialize(),
            ControlMeta::Other(other) => other.serialize(),
        };
        bytes.extend_from_slice(&meta);
        bytes.extend_from_slice(&self.stamp.serialize());
        bytes.extend_from_slice(&self.dst_socket_id.serialize());
        let info = match &self.info {
            ControlPacketInfo::Handshake(info) => info.serialize(),
            ControlPacketInfo::KeepAlive(info) => info.serialize(),
            ControlPacketInfo::Ack(info) => info.serialize(),
            ControlPacketInfo::Loss(info) => info.serialize(),
            ControlPacketInfo::Congestion(info) => info.serialize(),
            ControlPacketInfo::Shutdown(info) => info.serialize(),
            ControlPacketInfo::AckSquare(info) => info.serialize(),
            ControlPacketInfo::Drop(info) => info.serialize(),
            ControlPacketInfo::Err(info) => info.serialize(),
            ControlPacketInfo::Discover(info) => info.serialize(),
            ControlPacketInfo::Custom(info) => info.serialize(),
        };
        bytes.extend_from_slice(&info);
        bytes
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let control_type = ControlType::deserialize(bytes, start);
        let meta = match control_type {
            ControlType::Handshake => ControlMeta::Other(u16::deserialize(bytes, start)),
            ControlType::KeepAlive => ControlMeta::Other(u16::deserialize(bytes, start)),
            ControlType::Ack => ControlMeta::Seq(SequenceNumber::deserialize(bytes, start)),
            ControlType::Loss => ControlMeta::Seq(SequenceNumber::deserialize(bytes, start)),
            ControlType::Congestion => ControlMeta::Other(u16::deserialize(bytes, start)),
            ControlType::Shutdown => ControlMeta::Other(u16::deserialize(bytes, start)),
            ControlType::AckSquare => ControlMeta::Seq(SequenceNumber::deserialize(bytes, start)),
            ControlType::Drop => ControlMeta::Message(MessageNumber::deserialize(bytes, start)),
            ControlType::Err => ControlMeta::Other(u16::deserialize(bytes, start)),
            ControlType::Discover => ControlMeta::Other(u16::deserialize(bytes, start)),
            ControlType::Custom => ControlMeta::Other(u16::deserialize(bytes, start)),
        };
        let stamp = SystemTime::deserialize(bytes, start);
        let dst_socket_id = u16::deserialize(bytes, start);
        let info = match control_type {
            ControlType::Handshake => {
                ControlPacketInfo::Handshake(Handshake::deserialize(bytes, start))
            }
            ControlType::KeepAlive => {
                ControlPacketInfo::KeepAlive(KeepAlive::deserialize(bytes, start))
            }
            ControlType::Ack => ControlPacketInfo::Ack(Ack::deserialize(bytes, start)),
            ControlType::Loss => ControlPacketInfo::Loss(Loss::deserialize(bytes, start)),
            ControlType::Congestion => {
                ControlPacketInfo::Congestion(Congestion::deserialize(bytes, start))
            }
            ControlType::Shutdown => {
                ControlPacketInfo::Shutdown(Shutdown::deserialize(bytes, start))
            }
            ControlType::AckSquare => {
                ControlPacketInfo::AckSquare(AckSquare::deserialize(bytes, start))
            }
            ControlType::Drop => ControlPacketInfo::Drop(Drop::deserialize(bytes, start)),
            ControlType::Err => ControlPacketInfo::Err(Err::deserialize(bytes, start)),
            ControlType::Discover => {
                ControlPacketInfo::Discover(Discover::deserialize(bytes, start))
            }
            ControlType::Custom => ControlPacketInfo::Custom(Custom::deserialize(bytes, start)),
        };
        Self {
            control_type,
            meta,
            stamp,
            dst_socket_id,
            info,
        }
    }
}

impl Serial for ControlType {
    fn serialize(&self) -> Vec<u8> {
        let translation = match self {
            ControlType::Handshake => 0x0000u16,
            ControlType::KeepAlive => 0x0001u16,
            ControlType::Ack => 0x0002u16,
            ControlType::Loss => 0x0003u16,
            ControlType::Congestion => 0x0004u16,
            ControlType::Shutdown => 0x0005u16,
            ControlType::AckSquare => 0x0006u16,
            ControlType::Drop => 0x0007u16,
            ControlType::Err => 0x0008u16,
            ControlType::Discover => 0x0009u16,
            ControlType::Custom => 0x7fffu16,
        };
        translation.serialize()
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let translation = u16::deserialize(bytes, start) & 0x7f;
        match translation {
            0x0000u16 => ControlType::Handshake,
            0x0001u16 => ControlType::KeepAlive,
            0x0002u16 => ControlType::Ack,
            0x0003u16 => ControlType::Loss,
            0x0004u16 => ControlType::Congestion,
            0x0005u16 => ControlType::Shutdown,
            0x0006u16 => ControlType::AckSquare,
            0x0007u16 => ControlType::Drop,
            0x0008u16 => ControlType::Err,
            0x0009u16 => ControlType::Discover,
            0x7fffu16 => ControlType::Custom,
            _ => ControlType::Err,
        }
    }
}
