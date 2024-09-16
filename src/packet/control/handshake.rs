use std::{net::SocketAddr, time::{Duration, SystemTime, UNIX_EPOCH}};

use crate::{core::channel::MAX_PACKET_SIZE, serial::Serial, sha::Hash, utils::SequenceNumber};

use super::{ControlMeta, ControlPacket, ControlPacketInfo, ControlType};

pub const FLOW_CONTROL: u16 = 25600;
#[derive(Copy, Clone, Debug)]
pub struct Handshake {
    pub isn: SequenceNumber,
    pub req_type: ReqType,
    pub mss: u16,
    pub flow_control: u16,
    pub src_socket_id: u16, //my socket id
    pub cookie: u16,
    pub port: u16 //my in port
}

impl Handshake {
    pub fn cookie(mss: u16, flow_control: u16, socket_id: u16,time: SystemTime) -> (u16, u16) {
        let mut hash = Hash::new();
        hash.update(&mss.serialize());
        hash.update(&flow_control.serialize());
        hash.update(&socket_id.serialize());
        let minute = time
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            / 60;
        hash.update(&minute.serialize());
        let large_cookie = hash.finalize();
        let mut start = 0;
        let mut cookie = 0;
        //use the first half of the hash to get a cookie
        while start < large_cookie.len() / 2 {
            cookie ^= u16::deserialize(&large_cookie, &mut start);
        }

        //use the second half to generate an isn
        let mut isn = cookie;
        while start < large_cookie.len() {
            isn ^= u16::deserialize(&large_cookie, &mut start);
        }
        (cookie, isn)
    }

    pub fn new(req_type: ReqType, mss: u16, flow_control: u16, src_socket_id: u16, in_addr: SocketAddr) -> Self {
        //not cryptographically secure
        let (cookie, hash_isn) = Self::cookie(mss, flow_control, src_socket_id,SystemTime::now());
        let isn = SequenceNumber::new(hash_isn);
        let port = in_addr.port();
        Self {
            isn,
            req_type,
            mss,
            flow_control,
            src_socket_id,
            cookie,
            port,
        }
    }
    pub fn reply(src_socket_id: u16, dst_socket_id: u16, info: Self, in_addr: SocketAddr)-> ControlPacket{
        let mut isn = info.isn;
        isn.inc();
        let req_type = ReqType::Response;
        let mss = MAX_PACKET_SIZE;
        let flow_control = FLOW_CONTROL;
        let cookie = info.cookie;
        let port = in_addr.port();
        let info = Self{
            isn,
            req_type,
            mss,
            flow_control,
            src_socket_id,
            cookie,
            port,
        };
        ControlPacket{
            control_type: ControlType::Handshake,
            meta: ControlMeta::Other(0),
            stamp: SystemTime::now(),
            dst_socket_id,
            info:ControlPacketInfo::Handshake(info),
        }
    }
    pub fn validate(mss: u16, flow_control: u16, socket_id: u16,info:Self)->bool{
        //create a cookie from the packet
        let (cookie,isn) = Self::cookie(mss, flow_control, socket_id, SystemTime::now());
        if cookie!=info.cookie || SequenceNumber::new(isn) != info.isn{
            //try last cookie
            let (cookie,isn) = Self::cookie(mss, flow_control, socket_id, SystemTime::now()-Duration::from_millis(1000));
            if cookie!=info.cookie || SequenceNumber::new(isn) != info.isn{
                true
            }else{
                true
            }
        }else{
            true
        }
    }
}

impl Serial for Handshake {
    fn serialize(&self) -> Vec<u8> {
        let mut bytes = self.isn.serialize();

        match self.req_type {
            ReqType::Connection => bytes[0] |= 0x80,
            ReqType::Response => {}
        }
        bytes.extend_from_slice(&self.mss.serialize());
        bytes.extend_from_slice(&self.flow_control.serialize());
        bytes.extend_from_slice(&self.src_socket_id.serialize());
        bytes.extend_from_slice(&self.cookie.serialize());
        bytes.extend_from_slice(&self.port.serialize());
        bytes
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let control = bytes[*start];
        let req_type = match control & 0x80 {
            0x80 => ReqType::Connection,
            _ => ReqType::Response,
        };

        let isn = SequenceNumber::deserialize(bytes, start);
        let mss = u16::deserialize(bytes, start);
        let flow_control = u16::deserialize(bytes, start);
        let src_socket_id = u16::deserialize(bytes, start);
        let cookie = u16::deserialize(bytes, start);
        let port = u16::deserialize(bytes, start);
        Self {
            isn,
            mss,
            flow_control,
            req_type,
            src_socket_id,
            cookie,
            port
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ReqType {
    Connection,
    Response,
}
