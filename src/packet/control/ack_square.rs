use crate::serial::Serial;

#[derive(Copy, Clone, Debug)]
pub struct AckSquare {}

impl Default for AckSquare {
    fn default() -> Self {
        Self::new()
    }
}

impl AckSquare {
    pub fn new() -> Self {
        Self {}
    }
}

impl Serial for AckSquare {
    fn serialize(&self) -> Vec<u8> {
        vec![]
    }

    fn deserialize(_bytes: &[u8], _start: &mut usize) -> Self {
        Self {}
    }
}
