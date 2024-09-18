use std::{
    cmp::min,
    net::SocketAddr,
    sync::{Arc, RwLock},
    time::{Duration, SystemTime},
};

use crate::{
    congestion::CongestionController,
    core::{channel::NeonChannel, NeonStatus, SYN_INTERVAL},
    packet::{
        control::{ack::Ack, ControlPacket, ControlType},
        Packet,
    },
    utils::{MessageNumber, SequenceNumber, SequenceRange},
};

#[derive(Debug)]
pub struct NeonConnection {
    status: NeonStatus,
    isn: SequenceNumber,
    partner_id: u16,
    partner_in_addr: SocketAddr,
    partner_out_addr: SocketAddr,
    out_mss: u16,
    in_mss: u16,
    last_update: SystemTime,
    first_update: SystemTime, //this is for relative time processing
    closing: Arc<RwLock<bool>>,
    expiration_counter: usize,
    rtt: Duration,
    rtt_var: Duration,
    //congestion: CongestionController,
}
const MIN_EXPIRATION: usize = 300000;

//This is doing real control work
impl NeonConnection {
    pub fn new(
        isn: SequenceNumber,
        status: NeonStatus,
        partner_id: u16,
        partner_in_addr: SocketAddr,
        partner_out_addr: SocketAddr,
        out_mss: u16,
        in_mss: u16,
    ) -> Self {
        let last_update = SystemTime::now();
        let first_update = SystemTime::now();
        let closing = Arc::new(RwLock::new(false));
        //let congestion = CongestionController::new();

        let expiration_counter = 1;
        let rtt = SYN_INTERVAL * 10;
        let rtt_var = Duration::from_micros(rtt.as_micros() as u64 >> 1);
        let out = Self {
            isn,
            status,
            partner_id,
            partner_in_addr,
            partner_out_addr,
            out_mss,
            in_mss,
            last_update,
            first_update,
            closing,
            expiration_counter,
            rtt,
            rtt_var,
        };
        out
    }
    pub fn status(&self) -> NeonStatus {
        self.status
    }
    pub fn set_status(&mut self, status: NeonStatus) {
        self.status = status
    }
    pub fn partner_in_addr(&self)->SocketAddr{
        self.partner_in_addr
    }
    pub fn isn(&self)->SequenceNumber{
        self.isn
    }
    
    pub fn manage_keep_alive(&mut self)->bool{
        let mut exp_int = (self.expiration_counter
            * (self.rtt.as_micros() + 4 * self.rtt_var.as_micros()) as usize)
            + SYN_INTERVAL.as_micros() as usize;
        if exp_int < self.expiration_counter * MIN_EXPIRATION {
            exp_int = self.expiration_counter * MIN_EXPIRATION
        }
        let next_expiration_time = self.last_update + Duration::from_micros(exp_int as u64);
        match next_expiration_time.elapsed() {
            Ok(timeout) => {
                //if it's dead close the socket
                if self.expiration_counter > 16 && timeout > Duration::from_micros(500000) {
                    //self.congestion.on_timeout();
                    //TODO
                    *self.closing.write().unwrap() = true;
                    self.status = NeonStatus::Unhealthy(0x0001);
                    false
                }else{
                    true
                }
            }
            Err(_) => {false} //the time is just in the future
        }
    }
    pub fn create_ack(
        &mut self,
        ack_no: SequenceNumber,
        seq_no: SequenceNumber,
        buffer_size: u16,
        window: Duration,
        bandwidth: Duration,
    ) -> (SocketAddr, Packet) {
        let packet = Packet::Control(ControlPacket::ack(
            self.partner_id,
            ack_no,
            seq_no,
            self.rtt.as_millis() as u16,
            self.rtt_var.as_millis() as u16,
            buffer_size,
            window.as_millis() as u16,
            bandwidth.as_millis() as u16,
        ));
        (self.partner_in_addr, packet)
    }
    pub fn create_ack_square(&mut self, ack_no: SequenceNumber) -> (SocketAddr, Packet) {
        let packet = Packet::Control(ControlPacket::ack_square(self.partner_id, ack_no));
        (self.partner_in_addr, packet)
    }
    pub fn create_loss(&mut self, ranges: Vec<SequenceRange>) -> (SocketAddr, Packet) {
        let packet = Packet::Control(ControlPacket::loss(self.partner_id, ranges));
        (self.partner_in_addr, packet)
    }
    pub fn create_congestion(&mut self, factor: f64) -> (SocketAddr, Packet) {
        let encoding = factor.exp() as u16;
        let packet = Packet::Control(ControlPacket::congestion(self.partner_id, encoding));
        (self.partner_in_addr, packet)
    }
    pub fn create_keep_alive(&mut self) -> (SocketAddr, Packet) {
        let packet = Packet::Control(ControlPacket::keep_alive(self.partner_id));
        (self.partner_in_addr, packet)
    }
    pub fn create_shutdown(&mut self, code: u16) -> (SocketAddr, Packet) {
        let packet = Packet::Control(ControlPacket::shutdown(self.partner_id, code));
        (self.partner_in_addr, packet)
    }
    pub fn create_error(&mut self, code: u16) -> (SocketAddr, Packet) {
        let packet = Packet::Control(ControlPacket::error(self.partner_id, code));
        (self.partner_in_addr, packet)
    }
    pub fn create_drop(
        &mut self,
        msg_no: MessageNumber,
        ranges: SequenceRange,
    ) -> (SocketAddr, Packet) {
        let packet = Packet::Control(ControlPacket::drop(self.partner_id, msg_no, ranges));
        (self.partner_in_addr, packet)
    }
    pub fn create_discovery(&mut self) -> (SocketAddr, Packet) {
        let packet = Packet::Control(ControlPacket::discovery(
            self.partner_id,
            self.in_mss,
            self.out_mss,
        ));
        (self.partner_in_addr, packet)
    }

    pub fn establish(&mut self, in_mss: u16, out_mss: u16) {
        self.in_mss = min(self.in_mss, in_mss);
        self.out_mss = min(self.out_mss, out_mss);
        self.status = NeonStatus::Established;
    }
    pub fn error(&mut self, code: u16) {
        self.status = NeonStatus::Unhealthy(code)
    }

    pub fn validate(&mut self, addr: SocketAddr) -> bool {
        if self.partner_in_addr == addr {
            self.last_update = SystemTime::now();
            true
        } else {
            false
        }
    }
    pub fn mss(&self) -> (u16, u16) {
        (self.in_mss, self.out_mss)
    }

    pub fn negotiate(&mut self, stamp: SystemTime, partner_id: u16, port: u16) {
        self.first_update = stamp;
        self.partner_id = partner_id;
        self.status = NeonStatus::Negotiating;
        let partner_in_addr = SocketAddr::new(self.partner_out_addr.ip(), port);
        self.partner_in_addr = partner_in_addr;
    }
    pub fn partner_id(&self) -> u16 {
        self.partner_id
    }
    
}
