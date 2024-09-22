use core::cmp::min;

const CONSTANTS: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

const IV: [u8; 32] = [
    0x6a, 0x09, 0xe6, 0x67, 0xbb, 0x67, 0xae, 0x85, 0x3c, 0x6e, 0xf3, 0x72, 0xa5, 0x4f, 0xf5, 0x3a,
    0x51, 0x0e, 0x52, 0x7f, 0x9b, 0x05, 0x68, 0x8c, 0x1f, 0x83, 0xd9, 0xab, 0x5b, 0xe0, 0xcd, 0x19,
];

#[derive(Copy, Clone)]
struct State([u32; 8]);

struct W([u32; 16]);

impl W {
    fn new(input: &[u8]) -> Self {
        let mut w = [0u32; 16];
        for (i, e) in w.iter_mut().enumerate() {
            let marshal: [u8; 4] = input[i * 4..i * 4 + 4].try_into().unwrap_or_default();
            *e = u32::from_be_bytes(marshal); //divergence
        }
        Self(w)
    }
    fn ch(x: u32, y: u32, z: u32) -> u32 {
        (x & y) ^ (!x & z)
    }
    fn maj(x: u32, y: u32, z: u32) -> u32 {
        (x & y) ^ (x & z) ^ (y & z)
    }
    fn fsigma0(x: u32) -> u32 {
        x.rotate_right(2) ^ x.rotate_right(13) ^ x.rotate_right(22)
    }

    fn fsigma1(x: u32) -> u32 {
        x.rotate_right(6) ^ x.rotate_right(11) ^ x.rotate_right(25)
    }

    fn sigma0(x: u32) -> u32 {
        x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3)
    }

    fn sigma1(x: u32) -> u32 {
        x.rotate_right(17) ^ x.rotate_right(19) ^ (x >> 10)
    }

    fn m(&mut self, a: usize, b: usize, c: usize, d: usize) {
        let w = &mut self.0;
        w[a] = w[a]
            .wrapping_add(Self::sigma1(w[b]))
            .wrapping_add(w[c])
            .wrapping_add(Self::sigma0(w[d]));
    }

    fn expand(&mut self) {
        (0..16).for_each(|i| {
            self.m(i, (i + 14) & 15, (i + 9) & 15, (i + 1) & 15);
        });
    }
    fn f(&mut self, state: &mut State, i: usize, k: u32) {
        let t = &mut state.0;
        t[(16 - i + 7) & 7] = t[(16 - i + 7) & 7]
            .wrapping_add(Self::fsigma1(t[(16 - i + 4) & 7]))
            .wrapping_add(Self::ch(
                t[(16 - i + 4) & 7],
                t[(16 - i + 5) & 7],
                t[(16 - i + 6) & 7],
            ))
            .wrapping_add(k)
            .wrapping_add(self.0[i]);
        t[(16 - i + 3) & 7] = t[(16 - i + 3) & 7].wrapping_add(t[(16 - i + 7) & 7]);
        t[(16 - i + 7) & 7] = t[(16 - i + 7) & 7]
            .wrapping_add(Self::fsigma0(t[(16 - i) & 7]))
            .wrapping_add(Self::maj(
                t[(16 - i) & 7],
                t[(16 - i + 1) & 7],
                t[(16 - i + 2) & 7],
            ));
    }
    fn g(&mut self, state: &mut State, s: usize) {
        let offset = s * 16;
        (0..16).for_each(|i| self.f(state, i, CONSTANTS[offset + i]));
    }
}

impl State {
    fn new() -> Self {
        let mut t = [0u32; 8];
        for (i, e) in t.iter_mut().enumerate() {
            let marshal: [u8; 4] = IV[i * 4..i * 4 + 4].try_into().unwrap_or_default();
            *e = u32::from_be_bytes(marshal);
        }
        State(t)
    }
    fn add(&mut self, x: &State) {
        let sx = &mut self.0;
        let ex = &x.0;
        (0..8).for_each(|i| {
            sx[i] = sx[i].wrapping_add(ex[i]);
        });
    }
    fn store(&self, out: &mut [u8]) {
        for (i, &e) in self.0.iter().enumerate() {
            let bytes = e.to_be_bytes();
            out[i * 4..i * 4 + 4].copy_from_slice(&bytes);
        }
    }
    fn blocks(&mut self, mut input: &[u8]) -> usize {
        let mut t = *self;
        let mut inlen = input.len();
        while inlen >= 64 {
            let mut w = W::new(input);
            w.g(&mut t, 0);
            w.expand();
            w.g(&mut t, 1);
            w.expand();
            w.g(&mut t, 2);
            w.expand();
            w.g(&mut t, 3);
            t.add(self);
            self.0 = t.0;
            input = &input[64..];
            inlen -= 64;
        }
        inlen
    }
}
#[derive(Copy, Clone)]
pub struct Hash {
    state: State,
    w: [u8; 64],
    r: usize,
    len: usize,
}

impl Default for Hash {
    fn default() -> Self {
        Self::new()
    }
}

impl Hash {
    pub fn new() -> Hash {
        let state = State::new();
        let r = 0;
        let w = [0; 64];
        let len = 0;
        Hash { state, r, w, len }
    }
    pub fn update(&mut self, input: &[u8]) {
        let mut n = input.len();
        self.len += n;
        let av = 64 - self.r;
        let tc = min(n, av);
        self.w[self.r..self.r + tc].copy_from_slice(&input[0..tc]);
        self.r += tc;
        n -= tc;
        let pos = tc;
        if self.r == 64 {
            self.state.blocks(&self.w);
            self.r = 0;
        }
        if self.r == 0 && n > 0 {
            let rb = self.state.blocks(&input[pos..]);
            if rb > 0 {
                self.w[..rb].copy_from_slice(&input[pos + n - rb..]);
                self.r = rb;
            }
        }
    }
    pub fn finalize(mut self) -> [u8; 32] {
        let mut padded = [0u8; 128];
        padded[..self.r].copy_from_slice(&self.w[..self.r]);
        padded[self.r] = 0x80;
        let r = if self.r < 56 { 64 } else { 128 };
        let bits = self.len * 8;
        for i in 0..8 {
            padded[r - 8 + i] = (bits as u64 >> (56 - i * 8)) as u8;
        }
        self.state.blocks(&padded[..r]);
        let mut out = [0u8; 32];
        self.state.store(&mut out);
        out
    }
    pub fn hash(input: &[u8]) -> [u8; 32] {
        let mut h = Hash::new();
        h.update(input);
        h.finalize()
    }
}

#[derive(Clone)]
pub struct HMAC {
    ih: Hash,
    padded: [u8; 64],
}

impl HMAC {
    pub fn mac(input: impl AsRef<[u8]>, k: impl AsRef<[u8]>) -> [u8; 32] {
        let input = input.as_ref();
        let k = k.as_ref();
        let mut hk = [0u8; 32];
        let k2 = if k.len() > 64 {
            hk.copy_from_slice(&Hash::hash(k));
            &hk
        } else {
            k
        };
        let mut padded = [0x36; 64];
        for (p, &k) in padded.iter_mut().zip(k2.iter()) {
            *p ^= k;
        }
        let mut ih = Hash::new();
        ih.update(&padded[..]);
        ih.update(input);

        for p in padded.iter_mut() {
            *p ^= 0x6a;
        }
        let mut oh = Hash::new();
        oh.update(&padded[..]);
        oh.update(&ih.finalize());
        oh.finalize()
    }

    pub fn new(k: impl AsRef<[u8]>) -> HMAC {
        let k = k.as_ref();
        let mut hk = [0u8; 32];
        let k2 = if k.len() > 64 {
            hk.copy_from_slice(&Hash::hash(k));
            &hk
        } else {
            k
        };
        let mut padded = [0x36; 64];
        for (p, &k) in padded.iter_mut().zip(k2.iter()) {
            *p ^= k;
        }
        let mut ih = Hash::new();
        ih.update(&padded[..]);
        HMAC { ih, padded }
    }

    pub fn update(&mut self, input: &[u8]) {
        self.ih.update(input);
    }

    pub fn finalize(mut self) -> [u8; 32] {
        for p in self.padded.iter_mut() {
            *p ^= 0x6a;
        }
        let mut oh = Hash::new();
        oh.update(&self.padded[..]);
        oh.update(&self.ih.finalize());
        oh.finalize()
    }
}
