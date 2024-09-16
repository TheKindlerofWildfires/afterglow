use std::{
    collections::{HashMap, VecDeque},
    io::{Error, ErrorKind},
    net::SocketAddr,
    sync::{Arc, RwLock},
    thread,
    time::{Duration, SystemTime},
};

use channel::{NeonChannel, MAX_PACKET_SIZE};
use recv::recv_queue::RecvQueue;
use send::send_queue::SendQueue;

use crate::{
    connection::{self, NeonConnection},
    packet::{
        control::{
            handshake::{Handshake, ReqType, FLOW_CONTROL},
            ControlMeta, ControlPacket, ControlPacketInfo, ControlType,
        },
        data::DataPacket,
        Packet,
    },
    stream::NeonStream,
    utils::{MessageNumber, SequenceNumber, SequenceRange},
};

pub mod channel;
pub mod loss_list;
pub mod recv;
pub mod send;
const TIME_WINDOW_SIZE: usize = 1024;
pub const SYN_INTERVAL: Duration = Duration::from_micros(500);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum NeonStatus {
    Connecting,     //Sent a handshake requesting but no response yet
    Negotiating,    //Got a handshake response but the MSS isn't set
    Established,    //Connect is complete but it hasn't been exposed
    Queued,         //Stream is built but no one is working it
    Healthy,        //Connection is ready
    Unhealthy(u16), //Something went wrong, have an error code
}

/*
    New cardinality issue
        Each connection needs it's own seq number which is relevant to send/recv buffer
        so each connection needs it's own send/recv buffer
        which means it needs it's own send/recv list / queue
        so core probably needs to store a hash map of send/recv queues mapped by connection
        also base seq numbers aren't known till handshake.. so fake ones till then I think
*/
pub struct NeonCore {
    channel: Arc<RwLock<NeonChannel>>,
    connections: HashMap<u16, NeonConnection>,
    send: Arc<RwLock<SendQueue>>,
    recv: Arc<RwLock<RecvQueue>>,
    queued_streams: VecDeque<NeonStream>,
}

//this is mostly doing generic packet dispatch / recovery off the socket
impl NeonCore {
    pub fn new(channel: Arc<RwLock<NeonChannel>>) -> Self {
        let connections = HashMap::new();
        let queued_streams = VecDeque::new();
        let closing = Arc::new(RwLock::new(false));
        let send = Arc::new(RwLock::new(SendQueue::new(
            channel.clone(),
            closing.clone(),
        )));
        let recv = Arc::new(RwLock::new(RecvQueue::new(
            channel.clone(),
            closing.clone(),
        )));

        Self {
            channel,
            connections,
            queued_streams,
            send,
            recv,
        }
    }

    pub fn work(core: Arc<RwLock<NeonCore>>) {
        let thread_core = core.clone();
        //thread for working recv queue
        thread::spawn(move || {
            let channel = match thread_core.read() {
                Ok(tc) => tc.channel.clone(),
                Err(_) => return,
            };
            loop {
                match channel.read() {
                    Ok(channel) => {
                        let mut addr = "0.0.0.0:80".parse::<SocketAddr>().unwrap();
                        match channel.inbound.recv_from(&mut addr) {
                            Ok(packet) => {
                                drop(channel);
                                match thread_core.write() {
                                    Ok(mut tc) => tc.process_packet(addr, packet),
                                    Err(_) => return,
                                };
                            }
                            Err(_) => {}
                        }
                    }
                    Err(_) => return,
                }
                //TODO: condvars
                thread::sleep(Duration::from_millis(100));
                match thread_core.write() {
                    Ok(mut tc) => {
                        tc.manage_state();
                        tc.manage_streams(thread_core.clone());
                    }
                    Err(_) => return,
                };
            }
        });
        //thread for working send queue
        let thread_core = core.clone();
        thread::spawn(move || {
            loop {
                match thread_core.read() {
                    Ok(tc) => tc.manage_queue(),
                    Err(_) => {}
                }
                //TODO: condvars
                thread::sleep(Duration::from_millis(100));
            }
        });
    }
    pub fn manage_queue(&self) {
        let update = match self.send.read() {
            Ok(send) => send.next_time(),
            Err(_) => None,
        };
        match update {
            Some((socket_id, delay)) => {
                dbg!(socket_id, delay);
                thread::sleep(delay);
                let addr = match self.connections.get(&socket_id) {
                    Some(conn) => conn.partner_in_addr(),
                    None => return,
                };
                match self.send.write() {
                    Ok(mut send) => {
                        send.send_data(addr, socket_id);
                    }
                    Err(_) => {}
                };
            }
            None => {}
        }
    }
    pub fn manage_streams(&mut self, core: Arc<RwLock<NeonCore>>) {
        let streams = self
            .connections
            .iter_mut()
            .filter(|(_, connection)| connection.status() == NeonStatus::Established)
            .map(|(socket_id, connection)| {
                connection.set_status(NeonStatus::Queued);
                NeonStream::from_core(*socket_id, core.clone())
            })
            .collect::<Vec<_>>();
        self.queued_streams.extend(streams.into_iter())
    }
    pub fn manage_state(&mut self) {
        self.connections.iter_mut().for_each(|(_, connection)| {
            dbg!(connection.manage_ack());
        });
    }
    pub fn process_packet(&mut self, addr: SocketAddr, packet: Packet) {
        match packet {
            Packet::Control(packet) => self.process_control(addr, packet),
            Packet::Data(packet) => self.process_data(addr, packet),
        }
    }
    pub fn process_control(&mut self, addr: SocketAddr, packet: ControlPacket) {
        let socket_id = packet.dst_socket_id;
        //protect existing connections
        match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                if !connection.validate(addr) {
                    return;
                }
                if connection.manage_ack() {
                    self.send_ack(socket_id)
                }
            }
            None => {
                if packet.control_type == ControlType::Handshake {
                    self.new_connection(addr, packet);
                    return;
                }
            }
        };

        //should update the timeout here
        match packet.control_type {
            ControlType::Handshake => self.process_handshake(socket_id, packet),
            ControlType::KeepAlive => {}
            ControlType::Ack => self.process_ack(socket_id, packet),
            ControlType::Loss => self.process_loss(socket_id, packet),
            ControlType::Congestion => self.process_congestion(socket_id, packet),
            ControlType::Shutdown => self.process_shutdown(socket_id, packet),
            ControlType::AckSquare => self.process_ack_square(socket_id, packet),
            ControlType::Drop => self.process_drop(socket_id, packet),
            ControlType::Err => self.process_err(socket_id, packet),
            ControlType::Discover => self.process_discover(socket_id, packet),
            ControlType::Custom => {} //unsupported
        }
    }
    pub fn process_data(&mut self, addr: SocketAddr, packet: DataPacket) {
        let socket_id = packet.dst_socket_id;
        //protect existing connections
        let (_, out_mss) = match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                if !connection.validate(addr) {
                    return;
                }
                connection.mss()
            }
            None => return,
        };
        let losses = match self.recv.write() {
            Ok(mut recv) => recv.process_data(packet, out_mss),
            Err(_) => vec![],
        };
        if losses.len() > 0 {
            self.send_loss(socket_id, losses);
        }

        self.manage_state();
        //Toss the processing down to the recv queue / recv list
    }
    pub fn new_connection(&mut self, in_addr: SocketAddr, packet: ControlPacket) {
        let mut socket_id = packet.dst_socket_id;
        let stamp = packet.stamp;

        //should send a handshake packet back of type response, but only send it once (it will beacon if it doesn't get it)
        let info = match packet.info {
            ControlPacketInfo::Handshake(info) => info,
            _ => return,
        };
        let partner_in_addr = SocketAddr::new(in_addr.ip(), info.port);
        let valid = match info.req_type {
            ReqType::Connection => {
                //If it's a new request send back a response (use advertized socket, create new socket)
                match self.channel.read() {
                    Ok(channel) => {
                        socket_id = channel.inc_socket_id(); //A new request didn't know my socket

                        let local_out_addr = channel.inbound.addr;
                        let response_packet = Packet::Control(Handshake::reply(
                            socket_id,
                            info.src_socket_id,
                            info,
                            local_out_addr,
                        ));

                        match channel.send_to(partner_in_addr, response_packet) {
                            Ok(_) => true,
                            Err(_) => false,
                        }
                    }
                    Err(_) => false,
                }
            }
            ReqType::Response => {
                Handshake::validate(MAX_PACKET_SIZE, FLOW_CONTROL, socket_id, info)
            }
        };
        if valid {
            match self.connections.get_mut(&socket_id) {
                Some(connection) => {
                    connection.negotiate(stamp, info.src_socket_id, info.port);
                    //If there was already a connection update it
                    println!("A {}", socket_id);

                    self.send_discover(socket_id);
                }
                None => {
                    //If this is a first time set up a new connection
                    let connection = NeonConnection::new(
                        NeonStatus::Negotiating,
                        info.src_socket_id,
                        partner_in_addr,
                        in_addr,
                        MAX_PACKET_SIZE,
                        info.mss,
                        self.channel.clone(),
                    );
                    self.connections.insert(socket_id, connection);
                    match self.send.write() {
                        Ok(send) => send.register_connection(socket_id, info.isn),
                        Err(_) => {}
                    }
                    match self.recv.write() {
                        Ok(recv) => recv.register_connection(socket_id, info.isn),
                        Err(_) => {}
                    }
                    println!("C {}", socket_id);

                    self.send_discover(socket_id);
                }
            }
        }
    }

    //Send a handshake unless there is already a connection
    pub fn handshake(
        &mut self,
        req_type: ReqType,
        other_addr: SocketAddr,
        socket_id: u16,
    ) -> Result<(), Error> {
        let mss = MAX_PACKET_SIZE;
        let channel_lock = self.channel.read().unwrap();
        let local_in_addr = channel_lock.inbound.addr;
        let packet = Packet::Control(ControlPacket::handshake(
            u16::MAX, //We don't know the other socket
            req_type,
            mss,
            socket_id,
            local_in_addr,
        ));
        let isn = match &packet {
            Packet::Control(ctrl) => match ctrl.info {
                ControlPacketInfo::Handshake(handshake) => handshake.isn,
                _ => unreachable!(),
            },
            _ => {
                unreachable!()
            }
        };
        match &mut self.connections.get_mut(&socket_id) {
            //If this socket is already started
            Some(connection) => {
                //If it's connecting send another packet incase we dropped
                if connection.status() == NeonStatus::Connecting {
                    let channel_lock = self.channel.read().unwrap();
                    match channel_lock.send_to(other_addr, packet.clone()) {
                        Ok(_) => Err(Error::new(ErrorKind::NotConnected, "No response yet")),
                        Err(err) => Err(err),
                    }
                } else {
                    Ok(()) //this is an already established connections
                }
            }
            None => {
                //First time connection
                let connection = NeonConnection::new(
                    NeonStatus::Connecting,
                    0,
                    other_addr,
                    other_addr,
                    MAX_PACKET_SIZE,
                    MAX_PACKET_SIZE,
                    self.channel.clone(),
                );
                self.connections.insert(socket_id, connection);
                match self.send.write() {
                    Ok(send) => send.register_connection(socket_id, isn),
                    Err(_) => {}
                }
                match self.recv.write() {
                    Ok(recv) => recv.register_connection(socket_id, isn),
                    Err(_) => {}
                }
                let channel_lock = self.channel.read().unwrap();
                match channel_lock.send_to(other_addr, packet.clone()) {
                    Ok(_) => Err(Error::new(ErrorKind::NotConnected, "No response yet")),
                    Err(err) => Err(err),
                }
            }
        }
    }

    pub fn read_data(&mut self, socket_id: u16) -> Option<Vec<u8>> {
        match self.recv.read() {
            Ok(recv) => recv.read_data(socket_id),
            Err(_) => None,
        }
    }
    pub fn next_stream(&mut self) -> Option<NeonStream> {
        self.queued_streams.pop_front()
    }

    /*
    pub fn process_data(&mut self, packet: DataPacket) {
        self.last_update = SystemTime::now();
        self.congestion.write().unwrap().inc_pkt_cnt();
        let losses = self.recv.write().unwrap().process_data(packet);
        losses.into_iter().for_each(|(single, range)| {
            self.send_loss(single, range);
        });
        self.manage_state();
    }*/
    pub fn process_handshake(&mut self, socket_id: u16, packet: ControlPacket) {
        let stamp = packet.stamp;

        //should send a handshake packet back of type response, but only send it once (it will beacon if it doesn't get it)
        let info = match packet.info {
            ControlPacketInfo::Handshake(info) => info,
            _ => return,
        };
        match info.req_type {
            ReqType::Connection => {
                return;
            }
            ReqType::Response => {
                if Handshake::validate(MAX_PACKET_SIZE, FLOW_CONTROL, socket_id, info) {
                    match self.connections.get_mut(&socket_id) {
                        Some(connection) => {
                            connection.negotiate(stamp, info.src_socket_id, info.port)
                        }
                        None => {}
                    }
                    println!("B {}", socket_id);

                    self.send_discover(socket_id);
                }
            }
        };
    }
    pub fn send_data(
        &self,
        socket_id: u16,
        data: &[u8],
        ttl: Duration,
        order: bool,
    ) -> Result<(), Error> {
        let (partner_id, out_mss) = match self.connections.get(&socket_id) {
            Some(connection) => {
                let partner_id = connection.partner_id();
                let (_, out_mss) = connection.mss();
                (partner_id, out_mss)
            }
            None => return Err(Error::new(ErrorKind::Interrupted, "Poisoned")),
        };
        match self.send.write() {
            Ok(mut send) => {
                let cnt = send.push_data(socket_id, data, ttl, order, partner_id, out_mss);
                match self.connections.get(&socket_id) {
                    Some(connection) => {
                        let delay = connection.get_delay();
                        (0..cnt).for_each(|i| {
                            send.update(socket_id, SystemTime::now() + delay * i as u32);
                        });
                    }
                    None => {}
                };

                Ok(())
            }
            Err(_) => Err(Error::new(ErrorKind::Interrupted, "Poisoned")),
        }
    }
    pub fn send_ack(&mut self, socket_id: u16) {
        match self.recv.write() {
            Ok(recv) => {
                let ack = match recv.next_ack(socket_id) {
                    Some(ack) => ack,
                    None => return,
                };
                dbg!(ack);
                match self.connections.get_mut(&socket_id) {
                    Some(connection) => {
                        let should_ack = connection.should_ack(ack);
                        if should_ack {
                            let ack_no = match recv.inc_ack(socket_id) {
                                Some(ack_no) => ack_no,
                                None => {
                                    return;
                                }
                            };
                            let seq_no = SequenceNumber::new(0);/*match recv.inc_seq(socket_id) {
                                Some(seq_no) => seq_no,
                                None => {
                                    return;
                                }
                            };*/
                            let (buffer, window, bandwidth) = match recv.time_data(socket_id) {
                                Some(data) => data,
                                None => return,
                            };
                            let (addr, packet) =
                                connection.create_ack(seq_no, ack_no, buffer, window, bandwidth);
                            match self.send.write() {
                                Ok(send) => send.send_packet(addr, packet),
                                Err(_) => {}
                            }
                        }
                    }
                    None => return,
                };
            }
            Err(_) => return,
        };
    }
    pub fn send_ack_square(&mut self, socket_id: u16, ack_no: SequenceNumber) {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                let (addr, packet) = connection.create_ack_square(ack_no);
                match self.send.write() {
                    Ok(send) => send.send_packet(addr, packet),
                    Err(_) => {}
                }
            }
            None => {}
        }
    }
    pub fn send_loss(&mut self, socket_id: u16, ranges: Vec<SequenceRange>) {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                let (addr, packet) = connection.create_loss(ranges);
                match self.send.write() {
                    Ok(send) => send.send_packet(addr, packet),
                    Err(_) => {}
                }
            }
            None => {}
        }
    }
    pub fn send_congestion(&mut self, socket_id: u16, factor: f64) {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                let (addr, packet) = connection.create_congestion(factor);
                match self.send.write() {
                    Ok(send) => send.send_packet(addr, packet),
                    Err(_) => {}
                }
            }
            None => {}
        }
    }
    pub fn send_keep_alive(&mut self, socket_id: u16) {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                let (addr, packet) = connection.create_keep_alive();
                match self.send.write() {
                    Ok(send) => send.send_packet(addr, packet),
                    Err(_) => {}
                }
            }
            None => {}
        }
    }
    pub fn send_shutdown(&mut self, socket_id: u16, code: u16) {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                let (addr, packet) = connection.create_shutdown(code);
                match self.send.write() {
                    Ok(send) => send.send_packet(addr, packet),
                    Err(_) => {}
                }
            }
            None => {}
        }
    }
    pub fn send_error(&mut self, socket_id: u16, code: u16) {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                let (addr, packet) = connection.create_error(code);
                match self.send.write() {
                    Ok(send) => send.send_packet(addr, packet),
                    Err(_) => {}
                }
            }
            None => {}
        }
    }
    pub fn send_drop(&mut self, socket_id: u16, msg_no: MessageNumber, range: SequenceRange) {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                let (addr, packet) = connection.create_drop(msg_no, range);
                match self.send.write() {
                    Ok(send) => send.send_packet(addr, packet),
                    Err(_) => {}
                }
            }
            None => {}
        }
    }
    pub fn send_discover(&mut self, socket_id: u16) {
        match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                let (addr, packet) = connection.create_discovery();
                match self.send.write() {
                    Ok(send) => send.send_packet(addr, packet),
                    Err(_) => {}
                }
            }
            None => {}
        }
    }
    pub fn process_ack(&mut self, socket_id: u16, packet: ControlPacket) {
        let info = match packet.info {
            ControlPacketInfo::Ack(info) => info,
            _ => return,
        };
        let ack_no = match packet.meta {
            ControlMeta::Seq(other) => other,
            _ => return,
        };
        //Plan is to get a packet back from connection regarding an ack square if needed
        //And to get a status out of sender to update connection
        //handles ack square
        match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                if connection.ackd(ack_no, info) {
                    self.send_ack_square(socket_id, ack_no);
                }
            }
            None => return,
        }

        //validate ack
        match self.send.write() {
            Ok(mut binding) => binding.ack(socket_id, ack_no),
            Err(_) => {}
        }
    }
    pub fn process_loss(&mut self, socket_id: u16, packet: ControlPacket) {
        //update the sender loss list
        let info = match packet.info {
            ControlPacketInfo::Loss(info) => info,
            _ => return,
        };
        match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                info.loss_range
                    .iter()
                    .for_each(|range| connection.loss(*range));
            }
            None => todo!(),
        }
        match self.send.write() {
            Ok(mut binding) => {
                info.loss_range
                    .iter()
                    .for_each(|range| binding.loss(socket_id, *range));
            }
            Err(_) => {}
        }
    }
    pub fn process_congestion(&mut self, socket_id: u16, packet: ControlPacket) {
        //slow down the transmission rate
        let factor = match packet.meta {
            ControlMeta::Other(factor) => (factor as f64 + 1.0).log2(),
            _ => return,
        };
        match self.recv.write() {
            Ok(recv) => recv.update_delay(socket_id, factor),
            Err(_) => {}
        }
    }
    pub fn process_shutdown(&mut self, socket_id: u16, packet: ControlPacket) {
        let code = match packet.meta {
            ControlMeta::Other(other) => other,
            _ => return,
        };
        match self.connections.get(&socket_id) {
            Some(_) => self.send_shutdown(socket_id, code),
            None => todo!(),
        }

        self.connections.remove(&socket_id);

        match self.send.write() {
            Ok(mut binding) => binding.remove(socket_id),
            Err(_) => {}
        }
        match self.recv.write() {
            Ok(mut binding) => binding.remove(socket_id),
            Err(_) => {}
        }
    }
    pub fn process_ack_square(&mut self, socket_id: u16, packet: ControlPacket) {
        let ack_no = match packet.meta {
            ControlMeta::Seq(other) => other,
            _ => return,
        };
        /* 
        match self.send.read() {
            Ok(send) => send.ackd_square(ack_no),
            Err(_) => {}
        }*/
    }
    pub fn process_drop(&mut self, socket_id: u16, packet: ControlPacket) {
        let msg_no = match packet.meta {
            ControlMeta::Message(msg_no) => msg_no,
            _ => return,
        };
        let info = match packet.info {
            ControlPacketInfo::Drop(info) => info,
            _ => return,
        };
        match self.recv.write() {
            Ok(recv) => {
                recv.drop_msg(socket_id, msg_no, info.range);
            }
            Err(_) => {}
        };
    }
    pub fn process_err(&mut self, socket_id: u16, packet: ControlPacket) {
        let code = match packet.meta {
            ControlMeta::Other(other) => other,
            _ => return,
        };
        match self.connections.get_mut(&socket_id) {
            Some(connection) => connection.error(code),
            None => {}
        }
    }
    pub fn process_discover(&mut self, socket_id: u16, packet: ControlPacket) {
        //measures how much of the packet made it
        let info = match packet.info {
            ControlPacketInfo::Discover(info) => info,
            _ => return,
        };
        let meta = match packet.meta {
            ControlMeta::Other(other) => other,
            _ => return,
        };
        match self.connections.get_mut(&socket_id) {
            Some(connection) => connection.establish(info.data.len() as u16, meta),
            None => {}
        }
    }
}
