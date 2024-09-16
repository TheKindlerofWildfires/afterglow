use crate::serial::Serial;

#[derive(Copy,Clone,Debug)]
pub struct Shutdown{

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