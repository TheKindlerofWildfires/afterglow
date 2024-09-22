use std::fmt::Debug;
use crate::serial::Serial;

use super::handshake::ReqType;

#[derive(Clone)]
pub struct Discover {
    pub req_type: ReqType,
    pub data: Vec<u8>,
}

impl Discover{
    pub fn new(count:usize, req_type: ReqType)->Self{
        let char = match req_type{
            ReqType::Connection => 0xff,
            ReqType::Response => 0x7f,
        };
        let data = vec![char;count];
        Self{req_type, data}
    }
}

impl Serial for Discover {
    fn serialize(&self) -> Vec<u8> {
        self.data.clone()
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let data = bytes[*start..].to_vec();
        *start = bytes.len();
        let req_type = match data[0]{
            0xff=>{
                ReqType::Connection
            },
            _=>{
                ReqType::Response
            }
        };
        Self {req_type, data }
    }
}
impl Debug for Discover {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Discover")
            .field("req_type", &self.req_type)
            .field("len", &self.data.len())
            .finish()
    }
}