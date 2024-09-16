use crate::serial::Serial;

#[derive(Clone,Debug)]
pub struct Discover {
    pub data: Vec<u8>,
}

impl Discover{
    pub fn new(count:usize)->Self{
        let data = vec![0xff;count];
        Self{data}
    }
}

impl Serial for Discover {
    fn serialize(&self) -> Vec<u8> {
        self.data.clone()
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let data = bytes[*start..].to_vec();
        *start = bytes.len();
        Self { data }
    }
}
