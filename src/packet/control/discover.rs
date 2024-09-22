use crate::serial::Serial;

use super::handshake::ReqType;

#[derive(Clone,Debug)]
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
/*
    If a discovery request drops the connection never gets upgrade


*/