use crate::{serial::Serial, utils::SequenceRange};


#[derive(Copy,Clone,Debug)]
pub struct Drop {
    pub range: SequenceRange
}
impl Drop{
    pub fn new(range: SequenceRange)->Self{
        Self{
            range
        }
    }
}
impl Serial for Drop {
    fn serialize(&self) -> Vec<u8> {
        self.range.serialize()
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let range = SequenceRange::deserialize(bytes, start);
        Self {
            range
        }
    }
}
