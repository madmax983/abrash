use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

#[derive(Default)]
struct FastU64Hasher(u64);

impl Hasher for FastU64Hasher {
    #[inline]
    fn finish(&self) -> u64 { self.0 }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut x = self.0;
        for &b in bytes {
            x = x.rotate_left(8) ^ u64::from(b);
            x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        }
        self.0 = x;
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        let mut x = i;
        x ^= x >> 30;
        x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        x ^= x >> 27;
        x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
        x ^= x >> 31;
        self.0 = x;
    }
}

fn main() {
    let mut map: HashMap<u64, usize, BuildHasherDefault<FastU64Hasher>> = HashMap::default();
    map.insert(5, 10);
}
