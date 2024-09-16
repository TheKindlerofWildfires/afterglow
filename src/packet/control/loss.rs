use crate::{serial::Serial, utils::SequenceRange};

#[derive(Clone, Debug)]
pub struct Loss {
    pub loss_range: Vec<SequenceRange>,
}
impl Loss {
    pub fn new(loss_range: Vec<SequenceRange>) -> Self {
        Self {
            loss_range
        }
    }
}

impl Serial for Loss {
    fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.loss_range.iter().for_each(|loss|{
            bytes.extend_from_slice(&loss.serialize());
        });
        bytes
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let count = (bytes.len()-*start)/4;
        let loss_range = (0..count).map(|_|{
            SequenceRange::deserialize(bytes, start)
        }).collect();
        Self {
            loss_range
        }
    }
}
