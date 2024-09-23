use std::{
    collections::{HashMap, VecDeque},
    io::{Error, ErrorKind},
    net::SocketAddr,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

use channel::{NeonChannel, MAX_PACKET_SIZE};
use recv::recv_queue::RecvQueue;
use send::send_queue::{NeonPoll, SendQueue};

use crate::{
    connection::NeonConnection,
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
pub const SYN_INTERVAL: Duration = Duration::from_millis(10);

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
        let send = Arc::new(RwLock::new(SendQueue::new()));
        let recv = Arc::new(RwLock::new(RecvQueue::new()));

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
        //thread for working recv packets
        thread::spawn(move || {
            let channel = match thread_core.read() {
                Ok(tc) => tc.channel.clone(),
                Err(_) => return,
            };
            loop {
                //is there a way to poll on channel
                match channel.read() {
                    Ok(channel) => {
                        let mut addr = match "0.0.0.0:80".parse::<SocketAddr>() {
                            Ok(addr) => addr,
                            Err(_) => return,
                        };
                        if let Ok(packet) = channel.inbound.recv_from(&mut addr) {
                            drop(channel);
                            match thread_core.write() {
                                Ok(mut tc) => tc.process_packet(addr, packet),
                                Err(_) => return,
                            };
                        }
                    }
                    Err(_) => return,
                }
            }
        });

        let thread_core = core.clone();
        //thread for doing keep alive packets
        thread::spawn(move || {
            loop {
                //TODO: should sleep to next keep alive time but get interrupted if ???
                match thread_core.write() {
                    Ok(mut tc) => {
                        tc.manage_state();
                    }
                    Err(_) => return,
                };
                thread::sleep(Duration::from_millis(1000));
            }
        });

        //thread for working send queue
        let thread_core = core.clone();
        thread::spawn(move || {
            let poll = match thread_core.read() {
                Ok(tc) => tc.poll_send(),
                Err(_) => None,
            };

            loop {
                if let Ok(tc) = thread_core.read() {
                    tc.manage_queue()
                }
                //lot of words to say 'only pop when something gets added to the buffer'
                if let Some(arc) = poll.clone() {
                    let mut waiting_sockets = match arc.sockets.lock() {
                        Ok(ws) => ws,
                        Err(_) => return,
                    };
                    while waiting_sockets.is_empty() {
                        waiting_sockets = match arc.cond.wait(waiting_sockets) {
                            Ok(ws) => ws,
                            Err(_) => return,
                        };
                    }
                }
            }
        });
    }
    pub fn poll_send(&self) -> Option<Arc<NeonPoll>> {
        match self.send.read() {
            Ok(send) => send.poll(),
            Err(_) => None,
        }
    }
    pub fn manage_queue(&self) {
        let update = match self.send.read() {
            Ok(send) => send.next_time(),
            Err(_) => None,
        };
        if let Some((socket_id, delay)) = update {
            thread::sleep(delay);
            let addr = match self.connections.get(&socket_id) {
                Some(conn) => conn.partner_in_addr(),
                None => return,
            };
            let channel = match self.channel.read() {
                Ok(channel) => channel,
                Err(_) => return,
            };
            if let Ok(mut send) = self.send.write() {
                let _ = send.send_data(&channel, addr, socket_id);
            };
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
        self.queued_streams.extend(streams)
    }
    pub fn manage_state(&mut self) {
        //heal discovery portion
        let sockets = self
            .connections
            .iter()
            .filter(|(_, conn)| {
                let status = conn.status();
                status == NeonStatus::Negotiating
            })
            .map(|(socket_id, _)| *socket_id)
            .collect::<Vec<_>>();
        sockets
            .into_iter()
            .for_each(|socket_id| self.send_discover(socket_id, ReqType::Connection));
        //ack portion
        let sockets = self.connections.keys().copied().collect::<Vec<_>>();
        sockets.into_iter().for_each(|socket_id| {
            let next_ack = match self.recv.write() {
                Ok(recv) => recv.next_ack(socket_id),
                Err(_) => None,
            };
            if let Some((ack_no, seq_no)) = next_ack {
                self.send_ack(socket_id, ack_no, seq_no)
            }
        });
        //keep alive portion
        let sockets = self
            .connections
            .iter_mut()
            .filter_map(|(socket_id, connection)| {
                let (rtt, rtt_var) = match self.recv.read() {
                    Ok(recv) => match recv.time_data(*socket_id) {
                        Some(data) => (data.0, data.1),
                        None => (Duration::ZERO, Duration::ZERO),
                    },
                    Err(_) => (Duration::ZERO, Duration::ZERO),
                };
                if connection.should_keep_alive(rtt, rtt_var) {
                    Some(*socket_id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        sockets
            .into_iter()
            .for_each(|socket_id| self.send_keep_alive(socket_id));
    }
    pub fn process_packet(&mut self, addr: SocketAddr, packet: Packet) {
        //dbg!("Got a packet from ", addr, &packet);

        match packet {
            Packet::Control(packet) => self.process_control(addr, packet),
            Packet::Data(packet) => self.process_data(addr, packet),
        }
    }
    pub fn process_control(&mut self, addr: SocketAddr, packet: ControlPacket) {
        let socket_id = packet.dst_socket_id;
        //protect existing connections
        let connection_status = match self.connections.get_mut(&socket_id) {
            Some(connection) => {
                if !connection.validate(addr) && packet.control_type != ControlType::Handshake {
                    return;
                }
                connection.status()
            }
            None => {
                if packet.control_type == ControlType::Handshake {
                    self.new_connection(addr, packet);
                }
                return;
            }
        };
        let next_ack = match self.recv.write() {
            Ok(mut recv) => {
                recv.on_pkt(socket_id);
                if connection_status == NeonStatus::Connecting {
                    None
                } else {
                    recv.next_ack(socket_id)
                }
            }
            Err(_) => None,
        };
        if let Some((ack_no, seq_no)) = next_ack {
            self.send_ack(socket_id, ack_no, seq_no)
        }

        //should update the timeout here
        match packet.control_type {
            ControlType::Handshake => self.process_handshake(socket_id, packet),
            ControlType::KeepAlive => self.process_keep_alive(socket_id, packet),
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
        self.manage_state();
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
        if !losses.is_empty() {
            self.send_loss(socket_id, losses);
        } else {
            let next_ack = match self.recv.write() {
                Ok(mut recv) => {
                    recv.on_pkt(socket_id);
                    recv.next_ack(socket_id)
                }
                Err(_) => None,
            };
            if let Some((ack_no, seq_no)) = next_ack {
                self.send_ack(socket_id, ack_no, seq_no)
            }
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
        let mut isn = SequenceNumber::new(0);
        let valid = match info.req_type {
            ReqType::Connection => {
                //If it's a new request send back a response (use advertized socket, create new socket)
                match self.channel.read() {
                    Ok(channel) => {
                        socket_id = channel.inc_socket_id(); //A new request didn't know my socket

                        let local_out_addr = channel.outbound.addr;
                        let (out_isn, packet) =
                            Handshake::reply(socket_id, info.src_socket_id, info, local_out_addr);
                        isn = out_isn;
                        let response_packet = Packet::Control(packet);

                        channel.send_to(partner_in_addr, response_packet).is_ok()
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
                }
                None => {
                    //If this is a first time set up a new connection
                    let connection = NeonConnection::new(
                        isn,
                        NeonStatus::Negotiating,
                        info.src_socket_id,
                        partner_in_addr,
                        in_addr,
                        MAX_PACKET_SIZE,
                        info.mss,
                    );
                    self.connections.insert(socket_id, connection);
                    if let Ok(send) = self.send.write() {
                        send.register_connection(socket_id, isn)
                    }
                    if let Ok(recv) = self.recv.write() {
                        recv.register_connection(socket_id, info.isn)
                    }
                }
            }
            self.send_discover(socket_id, ReqType::Connection);
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
        let local_in_addr = match self.channel.read() {
            Ok(channel) => channel.inbound.addr,
            Err(_) => return Err(Error::new(ErrorKind::NotConnected, "Channel collapsed")),
        };
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
                    match self.channel.read() {
                        Ok(channel) => match channel.send_to(other_addr, packet.clone()) {
                            Ok(_) => Err(Error::new(ErrorKind::NotConnected, "No response yet")),
                            Err(err) => Err(err),
                        },
                        Err(_) => Err(Error::new(ErrorKind::NotConnected, "Channel collapsed")),
                    }
                } else {
                    Ok(()) //this is an already established connections
                }
            }
            None => {
                //First time connection
                let connection = NeonConnection::new(
                    isn,
                    NeonStatus::Connecting,
                    0,
                    other_addr,
                    other_addr,
                    MAX_PACKET_SIZE,
                    MAX_PACKET_SIZE,
                );
                self.connections.insert(socket_id, connection);
                match self.channel.read() {
                    Ok(channel) => match channel.send_to(other_addr, packet.clone()) {
                        Ok(_) => Err(Error::new(ErrorKind::NotConnected, "No response yet")),
                        Err(err) => Err(err),
                    },
                    Err(_) => Err(Error::new(ErrorKind::NotConnected, "Channel collapsed")),
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
    pub fn next_stream(&mut self, core: Arc<RwLock<NeonCore>>) -> Option<NeonStream> {
        //only clean up the queued connections when someone asks
        self.manage_streams(core.clone());
        self.queued_streams.pop_front()
    }

    pub fn process_handshake(&mut self, socket_id: u16, packet: ControlPacket) {
        let stamp = packet.stamp;

        //should send a handshake packet back of type response, but only send it once (it will beacon if it doesn't get it)
        let info = match packet.info {
            ControlPacketInfo::Handshake(info) => info,
            _ => return,
        };
        match info.req_type {
            ReqType::Connection => {}
            ReqType::Response => {
                if Handshake::validate(MAX_PACKET_SIZE, FLOW_CONTROL, socket_id, info) {
                    if let Some(connection) = self.connections.get_mut(&socket_id) {
                        connection.negotiate(stamp, info.src_socket_id, info.port);
                        if let Ok(recv) = self.recv.write() {
                            recv.register_connection(socket_id, info.isn)
                        }
                        if let Ok(send) = self.send.write() {
                            send.register_connection(socket_id, connection.isn())
                        }
                    }

                    self.send_discover(socket_id, ReqType::Connection);
                }
            }
        }
    }
    pub fn send_data(
        &mut self,
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
        let delay = match self.recv.read() {
            Ok(recv) => recv.delay(socket_id),
            Err(_) => Duration::ZERO,
        };
        let out = match self.send.write() {
            Ok(mut send) => {
                let cnt = send.push_data(socket_id, data, ttl, order, partner_id, out_mss);
                send.update(socket_id, cnt, delay);
                Ok(())
            }
            Err(_) => Err(Error::new(ErrorKind::Interrupted, "Poisoned")),
        };
        if let Some(connection)=self.connections.get_mut(&socket_id){
            connection.sent_packet();
        }
        out
    }
    pub fn send_ack(&mut self, socket_id: u16, ack_no: SequenceNumber, seq_no: SequenceNumber) {
        if let Ok(recv) = self.recv.write() {
            if let Some(connection) = self.connections.get_mut(&socket_id) {
                recv.sent_ack(socket_id, ack_no);
                let (rtt, rtt_var, buffer, window, bandwidth) = match recv.time_data(socket_id) {
                    Some(data) => data,
                    None => return,
                };
                let (addr, packet) = connection.create_ack(
                    ack_no,
                    seq_no,
                    rtt,
                    rtt_var,
                    buffer as u16,
                    window,
                    bandwidth,
                );
                let channel = match self.channel.read() {
                    Ok(channel) => channel,
                    Err(_) => return,
                };
                if let Ok(send) = self.send.write() {
                    let _ = send.send_packet(&channel, addr, packet);
                }
            }
        }
        if let Some(connection)=self.connections.get_mut(&socket_id){
            connection.sent_packet();
        }
    }
    pub fn send_ack_square(&mut self, socket_id: u16, ack_no: SequenceNumber) {
        if let Some(connection) = self.connections.get_mut(&socket_id) {
            let (addr, packet) = connection.create_ack_square(ack_no);
            let channel = match self.channel.read() {
                Ok(channel) => channel,
                Err(_) => return,
            };
            if let Ok(send) = self.send.write() {
                let _ = send.send_packet(&channel, addr, packet);
            }
        }
        if let Some(connection)=self.connections.get_mut(&socket_id){
            connection.sent_packet();
        }
    }
    pub fn send_loss(&mut self, socket_id: u16, ranges: Vec<SequenceRange>) {
        if let Some(connection) = self.connections.get_mut(&socket_id) {
            let (addr, packet) = connection.create_loss(ranges);
            let channel = match self.channel.read() {
                Ok(channel) => channel,
                Err(_) => return,
            };
            if let Ok(send) = self.send.write() {
                let _ = send.send_packet(&channel, addr, packet);
            }
        }
        if let Some(connection)=self.connections.get_mut(&socket_id){
            connection.sent_packet();
        }
    }
    pub fn send_congestion(&mut self, socket_id: u16, factor: f64) {
        if let Some(connection) = self.connections.get_mut(&socket_id) {
            let (addr, packet) = connection.create_congestion(factor);
            let channel = match self.channel.read() {
                Ok(channel) => channel,
                Err(_) => return,
            };
            if let Ok(send) = self.send.write() {
                let _ = send.send_packet(&channel, addr, packet);
            }
        }
        if let Some(connection)=self.connections.get_mut(&socket_id){
            connection.sent_packet();
        }
    }
    pub fn send_keep_alive(&mut self, socket_id: u16) {
        if let Ok(send) = self.send.write() {
            if send.keep_alive(socket_id) {
                if let Some(connection) = self.connections.get_mut(&socket_id) {
                    let (addr, packet) = connection.create_keep_alive();
                    let channel = match self.channel.read() {
                        Ok(channel) => channel,
                        Err(_) => return,
                    };
                    let _ = send.send_packet(&channel, addr, packet);
                }
            }
        }
        if let Some(connection)=self.connections.get_mut(&socket_id){
            connection.sent_packet();
        }
    }
    pub fn send_shutdown(&mut self, socket_id: u16, code: u16) {
        if let Some(connection) = self.connections.get_mut(&socket_id) {
            let (addr, packet) = connection.create_shutdown(code);
            let channel = match self.channel.read() {
                Ok(channel) => channel,
                Err(_) => return,
            };
            if let Ok(send) = self.send.write() {
                let _ = send.send_packet(&channel, addr, packet);
            }
        }
        if let Some(connection)=self.connections.get_mut(&socket_id){
            connection.sent_packet();
        }
    }
    pub fn send_error(&mut self, socket_id: u16, code: u16) {
        if let Some(connection) = self.connections.get_mut(&socket_id) {
            let (addr, packet) = connection.create_error(code);
            let channel = match self.channel.read() {
                Ok(channel) => channel,
                Err(_) => return,
            };
            if let Ok(send) = self.send.write() {
                let _ = send.send_packet(&channel, addr, packet);
            }
        }
        if let Some(connection)=self.connections.get_mut(&socket_id){
            connection.sent_packet();
        }
    }
    pub fn send_drop(&mut self, socket_id: u16, msg_no: MessageNumber, range: SequenceRange) {
        if let Some(connection) = self.connections.get_mut(&socket_id) {
            let (addr, packet) = connection.create_drop(msg_no, range);
            let channel = match self.channel.read() {
                Ok(channel) => channel,
                Err(_) => return,
            };
            if let Ok(send) = self.send.write() {
                let _ = send.send_packet(&channel, addr, packet);
            }
        }
        if let Some(connection)=self.connections.get_mut(&socket_id){
            connection.sent_packet();
        }
    }
    pub fn send_discover(&mut self, socket_id: u16, req_type: ReqType) {
        if let Some(connection) = self.connections.get_mut(&socket_id) {
            let (addr, packet) = connection.create_discovery(req_type);
            let channel = match self.channel.read() {
                Ok(channel) => channel,
                Err(_) => return,
            };
            if let Ok(send) = self.send.write() {
                let _ = send.send_packet(&channel, addr, packet);
            }
        }
        if let Some(connection)=self.connections.get_mut(&socket_id){
            connection.sent_packet();
        }
    }
    pub fn process_keep_alive(&mut self, socket_id: u16, _: ControlPacket) {
        //If keep alive but ack is wrong resend
        if let Ok(send) = self.send.write() {
            send.keep_alive(socket_id);
        }
        /*
        if let Ok(recv) = self.recv.write() {
            recv.drop_msg(socket_id, msg_no, info.range);
        };*/
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
        if let Ok(mut binding) = self.recv.write() {
            binding.on_ack(socket_id, ack_no, info)
        }

        if match self.send.write() {
            Ok(mut binding) => binding.ack(socket_id, ack_no),
            Err(_) => false,
        } {
            self.send_ack_square(socket_id, ack_no)
        }
    }
    pub fn process_loss(&mut self, socket_id: u16, packet: ControlPacket) {
        //update the sender loss list
        let info = match packet.info {
            ControlPacketInfo::Loss(info) => info,
            _ => return,
        };
        if let Ok(mut binding) = self.send.write() {
            info.loss_range
                .iter()
                .for_each(|range| binding.loss(socket_id, *range));
        }
        if let Ok(binding) = self.recv.read() {
            binding.loss(socket_id, info.loss_range);
        }
    }
    pub fn process_congestion(&mut self, socket_id: u16, packet: ControlPacket) {
        //slow down the transmission rate
        let factor = match packet.meta {
            ControlMeta::Other(factor) => (factor as f64 + 1.0).log2(),
            _ => return,
        };
        if let Ok(recv) = self.recv.write() {
            recv.update_delay(socket_id, factor)
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

        if let Ok(mut binding) = self.send.write() {
            binding.remove(socket_id)
        }
        if let Ok(mut binding) = self.recv.write() {
            binding.remove(socket_id)
        }
    }
    pub fn process_ack_square(&mut self, socket_id: u16, packet: ControlPacket) {
        let ack_no = match packet.meta {
            ControlMeta::Seq(other) => other,
            _ => return,
        };
        if let Ok(mut binding) = self.recv.write() {
            binding.ack_square(socket_id, ack_no)
        }

        //validate ack
        if let Ok(mut binding) = self.send.write() {
            binding.ack_square(socket_id, ack_no)
        }
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
        if let Ok(recv) = self.recv.write() {
            recv.drop_msg(socket_id, msg_no, info.range);
        };
    }

    pub fn process_err(&mut self, socket_id: u16, packet: ControlPacket) {
        let code = match packet.meta {
            ControlMeta::Other(other) => other,
            _ => return,
        };
        if let Some(connection) = self.connections.get_mut(&socket_id) {
            connection.error(code)
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
        if let Some(connection) = self.connections.get_mut(&socket_id) {
            connection.establish(info.data.len() as u16, meta);
            match info.req_type {
                ReqType::Connection => self.send_discover(socket_id, ReqType::Response),
                ReqType::Response => {}
            }
        }
    }
}
