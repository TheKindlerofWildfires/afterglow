use crate::serial::Serial;

#[derive(Copy, Clone, Debug)]
pub struct Err {}

impl Err {
    pub fn new() -> Self {
        Self {}
    }
}

impl Serial for Err {
    fn serialize(&self) -> Vec<u8> {
        vec![]
    }

    fn deserialize(_bytes: &[u8], _start: &mut usize) -> Self {
        Self {}
    }
}
