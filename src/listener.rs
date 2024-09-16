use std::{io::{Error, ErrorKind}, net::SocketAddr, sync::{Arc, RwLock}, thread, time::Duration};

use crate::{core::{channel::NeonChannel, NeonCore}, stream::NeonStream};

pub struct NeonListener {
    core: Arc<RwLock<NeonCore>>,
}
impl NeonListener {
    pub fn simplex(addr: SocketAddr) -> Result<Self, Error> {
        //create a socket
        let channel = match NeonChannel::simplex(addr.clone()) {
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
            match self.core.write(){
                Ok(mut core) => match core.next_stream() {
                    Some(stream) => {return Ok(stream);},
                    None => {}
                },
                Err(_) => return Err(Error::new(ErrorKind::Interrupted, "Poisoned")),
            }
            //TODO condvar
            thread::sleep(Duration::from_millis(1000));
        }
    }
    pub fn incoming(&mut self) {
        todo!()
    }
}