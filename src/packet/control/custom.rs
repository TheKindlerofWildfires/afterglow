use crate::serial::Serial;

#[derive(Clone,Debug)]
pub struct Custom {
    pub data: Vec<u8>,
}

impl Custom{
    pub fn new(data:Vec<u8>)->Self{
        Self{data}
    }
}

impl Serial for Custom {
    fn serialize(&self) -> Vec<u8> {
        self.data.clone()
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let data = bytes[*start..].to_vec();
        *start = bytes.len();
        Self { data }
    }
}
