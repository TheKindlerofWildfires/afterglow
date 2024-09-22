use std::{
    sync::OnceLock,
    time::{Duration, SystemTime},
};

static START: OnceLock<SystemTime> = OnceLock::new();

pub trait Serial {
    fn serialize(&self) -> Vec<u8>;
    fn deserialize(bytes: &[u8], start: &mut usize) -> Self;
}

impl Serial for u8 {
    fn serialize(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let next = *start + size_of::<u8>();
        let out = match bytes[*start..next].try_into() {
            Ok(res) => u8::from_be_bytes(res),
            Err(_) => u8::MAX,
        };
        *start = next;
        out
    }
}

impl Serial for u16 {
    fn serialize(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let next = *start + size_of::<u16>();
        let out = match bytes[*start..next].try_into() {
            Ok(res) => u16::from_be_bytes(res),
            Err(_) => u16::MAX,
        };
        *start = next;
        out
    }
}

impl Serial for u32 {
    fn serialize(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let next = *start + size_of::<u32>();
        let out = match bytes[*start..next].try_into() {
            Ok(res) => u32::from_be_bytes(res),
            Err(_) => u32::MAX,
        };
        *start = next;
        out
    }
}
impl Serial for u64 {
    fn serialize(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let next = *start + size_of::<u64>();
        let out = match bytes[*start..next].try_into() {
            Ok(res) => u64::from_be_bytes(res),
            Err(_) => u64::MAX,
        };
        *start = next;
        out
    }
}

impl Serial for i8 {
    fn serialize(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let next = *start + size_of::<u8>();
        let out = match bytes[*start..next].try_into() {
            Ok(res) => i8::from_be_bytes(res),
            Err(_) => i8::MAX,
        };
        *start = next;
        out
    }
}

impl Serial for i16 {
    fn serialize(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let next = *start + size_of::<u16>();
        let out = match bytes[*start..next].try_into() {
            Ok(res) => i16::from_be_bytes(res),
            Err(_) => i16::MAX,
        };
        *start = next;
        out
    }
}

impl Serial for i32 {
    fn serialize(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let next = *start + size_of::<u32>();
        let out = match bytes[*start..next].try_into() {
            Ok(res) => i32::from_be_bytes(res),
            Err(_) => i32::MAX,
        };
        *start = next;
        out
    }
}
impl Serial for i64 {
    fn serialize(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let next = *start + size_of::<u64>();
        let out = match bytes[*start..next].try_into() {
            Ok(res) => i64::from_be_bytes(res),
            Err(_) => i64::MAX,
        };
        *start = next;
        out
    }
}

impl Serial for f32 {
    fn serialize(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let next = *start + size_of::<f32>();
        let out = match bytes[*start..next].try_into() {
            Ok(res) => f32::from_be_bytes(res),
            Err(_) => f32::MAX,
        };
        *start = next;
        out
    }
}
impl Serial for f64 {
    fn serialize(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let next = *start + size_of::<f64>();
        let out = match bytes[*start..next].try_into() {
            Ok(res) => f64::from_be_bytes(res),
            Err(_) => f64::MAX,
        };
        *start = next;
        out
    }
}

//This gives about 18 hours of lifetime before TTL shuffle happens
//This is fixed by a 'timing packet' that tells the other side to drop all after the sync
impl Serial for SystemTime {
    fn serialize(&self) -> Vec<u8> {
        let boot_time = START.get_or_init(|| {
            match SystemTime::now().checked_sub(Duration::from_millis(60000)) {
                Some(sys) => sys,
                None => SystemTime::now(),
            }
        });
        match self.duration_since(*boot_time) {
            Ok(dur) => ((dur.as_secs() % (u16::MAX as u64)) as u16).serialize(),
            Err(err) => ((err.duration().as_secs() % (u16::MAX as u64)) as u16).serialize(),
        }
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let boot_time = START.get_or_init(|| {
            match SystemTime::now().checked_sub(Duration::from_millis(60000)) {
                Some(sys) => sys,
                None => SystemTime::now(),
            }
        });
        let seconds = u16::deserialize(bytes, start);
        *boot_time + Duration::from_secs(seconds.into())
    }
}

impl<T: Serial + Default + Copy, const N: usize> Serial for [T; N] {
    fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(N * size_of::<T>());
        self.iter()
            .for_each(|i| bytes.extend_from_slice(&i.serialize()));
        bytes
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let mut out = [T::default(); N];
        out.iter_mut().for_each(|i| {
            *i = T::deserialize(bytes, start);
        });
        out
    }
}
