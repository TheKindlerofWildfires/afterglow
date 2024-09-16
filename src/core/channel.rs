use crate::{
    packet::{Packet, HEADER_SIZE},
    serial::Serial,
};
use std::{
    io::Error,
    net::{SocketAddr, UdpSocket},
    num::Wrapping,
    sync::Arc,
    sync::Mutex,
    time::Duration,
};

pub const MAX_PACKET_SIZE: u16 = 128;
static SOCKET_COUNTER: Mutex<Wrapping<u16>> = Mutex::new(Wrapping(0));
pub struct NeonChannel {
    pub outbound: Arc<NeonSocket>,
    pub inbound: Arc<NeonSocket>,
}
pub struct NeonSocket {
    pub addr: SocketAddr,
    socket: UdpSocket,
    direction: SocketDirection,
}
pub enum SocketDirection {
    In,
    Out,
    Shared,
}

//Wrapper for two way channels
impl NeonChannel {
    pub fn simplex(sock_addr: SocketAddr) -> Result<Self, Error> {
        let dual = match NeonSocket::open(sock_addr, SocketDirection::Shared) {
            Ok(connection) => Arc::new(connection),
            Err(err) => return Err(err),
        };
        Ok(Self {
            outbound: dual.clone(),
            inbound: dual.clone(),
        })
    }

    pub fn duplex(out_addr: SocketAddr, in_addr: SocketAddr) -> Result<Self, Error> {
        let outbound = match NeonSocket::open(out_addr, SocketDirection::Shared) {
            Ok(connection) => Arc::new(connection),
            Err(err) => return Err(err),
        };
        let inbound = match NeonSocket::open(in_addr, SocketDirection::Shared) {
            Ok(connection) => Arc::new(connection),
            Err(err) => return Err(err),
        };
        Ok(Self { outbound, inbound })
    }

    pub fn inc_socket_id(&self) -> u16 {
        match SOCKET_COUNTER.lock() {
            Ok(mut ctr) => {
                let socket_id = ctr.0;
                *ctr = *ctr + Wrapping(1u16);
                socket_id
            }
            Err(_) => 0xffff,
        }
    }

    pub fn send_to(&self, addr: SocketAddr, packet: Packet) -> Result<usize, Error> {
        self.outbound.send_to(addr, packet)
    }
    pub fn recv_from(&self, addr: &mut SocketAddr) -> Result<Packet, Error> {
        self.inbound.recv_from(addr)
    }
}

impl NeonSocket {
    pub fn open(addr: SocketAddr, direction: SocketDirection) -> Result<Self, Error> {
        let socket = match UdpSocket::bind(addr) {
            Ok(socket) => socket,
            Err(err) => return Err(err),
        };

        //set nonblockng

        match socket.set_nonblocking(true) {
            Ok(_) => {}
            Err(err) => return Err(err),
        }

        //set read timeout
        match socket.set_read_timeout(Some(Duration::from_millis(100))) {
            Ok(_) => {}
            Err(err) => return Err(err),
        }

        Ok(Self {
            addr,
            socket,
            direction,
        })
    }

    pub fn send_to(&self, addr: SocketAddr, packet: Packet) -> Result<usize, Error> {
        let socket = match self.direction {
            SocketDirection::In => unreachable!(),
            SocketDirection::Out => &self.socket,
            SocketDirection::Shared => &self.socket,
        };
        socket.send_to(&packet.serialize(), addr)
    }
    pub fn recv_from(&self, addr: &mut SocketAddr) -> Result<Packet, Error> {
        let socket = match self.direction {
            SocketDirection::In => &self.socket,
            SocketDirection::Out => unreachable!(),
            SocketDirection::Shared => &self.socket,
        };
        //the maximum allowed packet size
        let mut bytes = vec![0u8; HEADER_SIZE + MAX_PACKET_SIZE as usize];
        let (count, recv_addr) = match socket.recv_from(&mut bytes) {
            Ok(res) => res,
            Err(err) => return Err(err),
        };
        let packet = Packet::deserialize(&bytes[..count], &mut 0);
        *addr = recv_addr;
        Ok(packet)
    }
}
