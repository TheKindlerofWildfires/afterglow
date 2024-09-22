use std::{
    io::{Error, ErrorKind},
    net::SocketAddr,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

use crate::{
    core::{
        channel::NeonChannel,
        NeonCore,
    },
    packet::control::handshake::ReqType,
};

pub struct NeonStream {
    core: Arc<RwLock<NeonCore>>,
    socket_id: u16,
}

impl NeonStream {
    pub fn simplex(
        addr: SocketAddr,
        max_attempts: usize,
        timeout: Duration,
        other: SocketAddr,
    ) -> Result<Self, Error> {
        //Step 1: Create the channels
        let (socket_id, channel) = match NeonChannel::simplex(addr) {
            Ok(channel) => {
                let socket_id = channel.inc_socket_id();
                (socket_id, Arc::new(RwLock::new(channel)))
            }
            Err(err) => return Err(err),
        };

        //Step 2: Create a core with max mss
        let core = Arc::new(RwLock::new(NeonCore::new(channel)));
        NeonCore::work(core.clone());
        //Step 3: Handshake to establish connection (should timeout loop)
        match Self::handshake(max_attempts, timeout, other, core.clone(), socket_id) {
            Ok(_) => {}
            Err(err) => return Err(err),
        }

        Ok(Self { core, socket_id })
    }
    pub fn duplex(
        out_addr: SocketAddr,
        in_addr: SocketAddr,
        max_attempts: usize,
        timeout: Duration,
        other: SocketAddr,
    ) -> Result<Self, Error> {
        //Step 1: Create the channels
        let (socket_id, channel) = match NeonChannel::duplex(out_addr,in_addr) {
            Ok(channel) => {
                let socket_id = channel.inc_socket_id();
                (socket_id, Arc::new(RwLock::new(channel)))
            }
            Err(err) => return Err(err),
        };

        //Step 2: Create a core with max mss
        let core = Arc::new(RwLock::new(NeonCore::new(channel)));
        NeonCore::work(core.clone());
        //Step 3: Handshake to establish connection (should timeout loop)
        match Self::handshake(max_attempts, timeout, other, core.clone(), socket_id) {
            Ok(_) => {}
            Err(err) => return Err(err),
        }

        Ok(Self { core, socket_id })
    }
    pub fn from_core(socket_id: u16, core: Arc<RwLock<NeonCore>>)->Self{
        Self { core, socket_id }
    }

    fn handshake(
        max_attempts: usize,
        timeout: Duration,
        other: SocketAddr,
        core: Arc<RwLock<NeonCore>>,
        new_socket_id: u16,
    ) -> Result<(), Error> {
        let mut count = 0;
        loop {
            let err = match core.write() {
                Ok(mut core) => match core.handshake(ReqType::Connection, other, new_socket_id) {
                    Ok(()) => {
                        return Ok(());
                    }
                    Err(err) => Err(err),
                },
                Err(_) => return Err(Error::new(ErrorKind::Interrupted, "Poisoned")),
            };
            count += 1;

            if count >= max_attempts {
                return err;
            }
            thread::sleep(timeout);
        }
    }

    pub fn read(&self) -> Vec<u8> {
        let socket_id = self.socket_id;
        loop {
            if let Ok(mut core) = self.core.write() { if let Some(data) = core.read_data(socket_id) {
                return data;
            } }
        }
    }
    pub fn write(&self, bytes: &[u8], ttl: Duration, order: bool) -> Result<(), Error> {
        let socket_id = self.socket_id;
        //, addr: SocketAddr, data: &[u8], ttl: Duration, order: bool
        match self.core.write() {
            Ok(core) => core.send_data(socket_id, bytes, ttl, order),
            Err(_) => Err(Error::new(ErrorKind::Interrupted, "Poisoned")),
        }
    }
}
