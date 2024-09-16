use crate::serial::Serial;

#[derive(Copy,Clone,Debug)]
pub struct Congestion{}

impl Congestion {
    pub fn new() -> Self {
        Self {}
    }
}

impl Serial for Congestion{
    fn serialize(&self)->Vec<u8> {
        vec![]
    }

    fn deserialize(_bytes: &[u8], _start: &mut usize)->Self {
        Self{}
    }
}