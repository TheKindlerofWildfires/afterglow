use crate::serial::Serial;

#[derive(Copy,Clone,Debug)]
pub struct Shutdown{

}
impl Default for Shutdown {
    fn default() -> Self {
        Self::new()
    }
}

impl Shutdown{
    pub fn new()->Self{
        Self{}
    }
}

impl Serial for Shutdown{
    fn serialize(&self)->Vec<u8> {
        vec![]
    }

    fn deserialize(_bytes: &[u8], _start: &mut usize)->Self {
        Self{}
    }
}