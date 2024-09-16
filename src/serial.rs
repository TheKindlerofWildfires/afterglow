use std::{sync::OnceLock, time::{Duration, SystemTime}};

static START: OnceLock<SystemTime> = OnceLock::new();

pub trait Serial {
    fn serialize(&self)->Vec<u8>;
    fn deserialize(bytes: &[u8], start: &mut usize)->Self;
}

impl Serial for u8{
    fn serialize(&self)->Vec<u8> {
        self.to_be_bytes().to_vec()
    }

    fn deserialize(bytes: &[u8], start: &mut usize) ->Self {
        let next = *start+size_of::<u8>();
        let out = u8::from_be_bytes(bytes[*start..next].try_into().unwrap());
        *start = next;
        out
    }
}

impl Serial for u16{
    fn serialize(&self)->Vec<u8> {
        self.to_be_bytes().to_vec()
    }

    fn deserialize(bytes: &[u8], start: &mut usize) ->Self {
        let next = *start+size_of::<u16>();
        let out = u16::from_be_bytes(bytes[*start..next].try_into().unwrap());
        *start = next;
        out
    }
}

impl Serial for u32{
    fn serialize(&self)->Vec<u8> {
        self.to_be_bytes().to_vec()
    }

    fn deserialize(bytes: &[u8], start: &mut usize) ->Self {
        let next = *start+size_of::<u32>();
        let out = u32::from_be_bytes(bytes[*start..next].try_into().unwrap());
        *start = next;
        out
    }
}
impl Serial for u64{
    fn serialize(&self)->Vec<u8> {
        self.to_be_bytes().to_vec()
    }

    fn deserialize(bytes: &[u8], start: &mut usize) ->Self {
        let next = *start+size_of::<u64>();
        let out = u64::from_be_bytes(bytes[*start..next].try_into().unwrap());
        *start = next;
        out
    }
}

impl Serial for f32{
    fn serialize(&self)->Vec<u8> {
        self.to_be_bytes().to_vec()
    }

    fn deserialize(bytes: &[u8], start: &mut usize) ->Self {
        let next = *start+size_of::<f32>();
        let out = f32::from_be_bytes(bytes[*start..next].try_into().unwrap());
        *start = next;
        out
    }
}
impl Serial for f64{
    fn serialize(&self)->Vec<u8> {
        self.to_be_bytes().to_vec()
    }

    fn deserialize(bytes: &[u8], start: &mut usize) ->Self {
        let next = *start+size_of::<f64>();
        let out = f64::from_be_bytes(bytes[*start..next].try_into().unwrap());
        *start = next;
        out
    }
}

//This gives about 18 hours of lifetime before TTL shuffle happens
//This is fixed by a 'timing packet' that tells the other side to drop all after the sync
impl Serial for SystemTime{
    fn serialize(&self)->Vec<u8> {
        let boot_time = START.get_or_init(||{
            SystemTime::now().checked_sub(Duration::from_millis(60000)).unwrap()
        });
        ((self.duration_since(*boot_time).unwrap().as_secs()%(u16::MAX as u64)) as u16).serialize()
    }

    fn deserialize(bytes: &[u8], start: &mut usize)->Self {
        let boot_time = START.get_or_init(||{
            SystemTime::now().checked_sub(Duration::from_millis(60000)).unwrap()
        });
        let seconds = u16::deserialize(bytes, start);
        *boot_time+Duration::from_secs(seconds.into())
    }
}

impl<T:Serial+Default+Copy, const N:usize> Serial for [T;N]{
    fn serialize(&self)->Vec<u8> {
        let mut bytes = Vec::with_capacity(N*size_of::<T>());
        self.iter().for_each(|i|{
            bytes.extend_from_slice(&i.serialize())
        });
        bytes
    }

    fn deserialize(bytes: &[u8], start: &mut usize)->Self {
        let mut out = [T::default();N];
        out.iter_mut().for_each(|i|{
            *i = T::deserialize(bytes, start);
        });
        out
    }
}