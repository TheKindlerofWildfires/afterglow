use crate::serial::Serial;

#[derive(Copy,Clone,Debug)]
pub struct KeepAlive {}

impl Default for KeepAlive {
    fn default() -> Self {
        Self::new()
    }
}

impl KeepAlive{
    pub fn new()->Self{
        Self{}
    }
}

impl Serial for KeepAlive {
    fn serialize(&self) -> Vec<u8> {
        vec![]
    }

    fn deserialize(_bytes: &[u8], _start: &mut usize) -> Self {
        Self {}
    }
}
