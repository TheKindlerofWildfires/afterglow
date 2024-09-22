use std::{io::{Error, ErrorKind}, net::SocketAddr, sync::{Arc, RwLock}, thread, time::Duration};

use crate::{core::{channel::NeonChannel, NeonCore}, stream::NeonStream};

pub struct NeonListener {
    core: Arc<RwLock<NeonCore>>,
}
impl NeonListener {
    pub fn simplex(addr: SocketAddr) -> Result<Self, Error> {
        //create a socket
        let channel = match NeonChannel::simplex(addr) {
            Ok(channel) => Arc::new(RwLock::new(channel)),
            Err(err) => return Err(err),
        };

        let core = Arc::new(RwLock::new(NeonCore::new(channel.clone())));
        NeonCore::work(core.clone());
        Ok(Self { core })
    }
    pub fn duplex(out_addr: SocketAddr, in_addr: SocketAddr) -> Result<Self, Error> {
        //create a socket
        let channel = match NeonChannel::duplex(out_addr,in_addr) {
            Ok(channel) => Arc::new(RwLock::new(channel)),
            Err(err) => return Err(err),
        };

        let core = Arc::new(RwLock::new(NeonCore::new(channel.clone())));
        NeonCore::work(core.clone());
        Ok(Self { core })
    }
    pub fn accept(&mut self) -> Result<NeonStream, Error> {
        loop {
            //check if there is a queued stream
            let thread_core = self.core.clone();
            match self.core.write(){
                Ok(mut core) => if let Some(stream) = core.next_stream(thread_core) {return Ok(stream);},
                Err(_) => return Err(Error::new(ErrorKind::Interrupted, "Poisoned")),
            }
            //TODO condvar
            thread::sleep(Duration::from_millis(100));
        }
    }
    pub fn incoming(&mut self) {
        todo!()
    }
}