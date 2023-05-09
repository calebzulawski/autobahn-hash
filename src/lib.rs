#![feature(portable_simd)]

use core::simd::{simd_swizzle, u32x8, u64x4, u8x32};

/// A hash instance.
#[derive(Copy, Clone)]
pub struct AutobahnHash {
    v0: u64x4,
    v1: u64x4,
    mul0: u64x4,
    mul1: u64x4,
}

unsafe trait SafeTransmute<To>: Sized {
    fn transmute_to(self) -> To {
        unsafe { core::mem::transmute_copy(&self) }
    }
}

unsafe impl SafeTransmute<u8x32> for u64x4 {}
unsafe impl SafeTransmute<u32x8> for u64x4 {}
unsafe impl SafeTransmute<u64x4> for u8x32 {}
unsafe impl SafeTransmute<u64x4> for u32x8 {}

fn zipper_merge(x: u64x4) -> u64x4 {
    const INDEX: [usize; 32] = {
        let half_index = [7, 8, 6, 9, 13, 10, 4, 11, 0, 15, 1, 14, 5, 2, 12, 3];
        let mut index = [0; 32];
        let mut i = 0;
        while i < 32 {
            index[i] = half_index[i % 16];
            i += 1;
        }
        index
    };

    let x: u8x32 = x.transmute_to();
    simd_swizzle!(x, INDEX).transmute_to()
}

fn mul32(x: u64x4, y: u64x4) -> u64x4 {
    let x: u32x8 = x.transmute_to();
    let y: u32x8 = y.transmute_to();
    (x * y).transmute_to()
}

fn permute(x: u64x4) -> u64x4 {
    let x: u32x8 = x.transmute_to();
    let x = simd_swizzle!(x, [2, 3, 0, 1, 6, 7, 4, 5]);
    x.transmute_to()
}

fn remainder(bytes: &[u8]) -> [u8; 32] {
    let mut packet: [u8; 32] = [0u8; 32];
    let size_mod4 = bytes.len() & 3;
    let remaining = bytes.len() & !3;
    let remainder = &bytes[remaining..];
    let size = bytes.len() as u64;

    packet[..remaining].copy_from_slice(&bytes[..remaining]);
    if size & 16 != 0 {
        packet[28..32].copy_from_slice(&bytes[size_mod4 - 4..size_mod4]);
    } else if size_mod4 != 0 {
        packet[16] = remainder[0];
        packet[16 + 1] = remainder[size_mod4 >> 1];
        packet[16 + 2] = remainder[size_mod4 - 1];
    }

    packet
}

fn bytes_as_packet(bytes: [u8; 32]) -> [u64; 4] {
    let mut packet = [0; 4];
    for (i, chunk) in bytes.chunks(8).enumerate() {
        packet[i] = u64::from_le_bytes(chunk.try_into().unwrap());
    }
    packet
}

impl AutobahnHash {
    /// Create a new `AutobahnHash` with the given key.
    pub fn new(key: [u64; 4]) -> Self {
        let key = u64x4::from_array(key);
        let mul0 = u64x4::from_array([
            0xdbe6d5d5fe4cce2f,
            0xa4093822299f31d0,
            0x13198a2e03707344,
            0x243f6a8885a308d3,
        ]);
        let mul1 = u64x4::from_array([
            0x3bd39e10cb0ef593,
            0xc0acf169b5f18a8c,
            0xbe5466cf34e90c6c,
            0x452821e638d01377,
        ]);
        let v0 = mul0 ^ key;
        let v1 = mul1 ^ (key >> u64x4::splat(32) | key << u64x4::splat(32));
        Self { v0, v1, mul0, mul1 }
    }

    pub fn write(&mut self, packet: [u64; 4]) {
        let packet = u64x4::from_array(packet);
        self.v1 += packet;
        self.v1 += self.mul0;
        self.mul0 ^= mul32(self.v1, self.v0 >> u64x4::splat(32));
        self.v0 += self.mul1;
        self.mul1 ^= mul32(self.v0, self.v1 >> u64x4::splat(32));
        self.v0 += zipper_merge(self.v1);
        self.v1 += zipper_merge(self.v0);
    }

    fn finish(&mut self, remainder: &[u8]) {
        fn rotate_32_by(count: u64, lanes: &mut u64x4) {
            for lane in lanes.as_mut_array() {
                let half0: u32 = *lane as u32;
                let half1: u32 = (*lane >> 32) as u32;
                *lane = u64::from((half0 << count) | (half0 >> (32 - count)));
                *lane |= u64::from((half1 << count) | (half1 >> (32 - count))) << 32;
            }
        }

        assert!(remainder.len() < 32, "remainder must be less than 32 bytes");
        if !remainder.is_empty() {
            let size = remainder.len() as u64;
            self.v0 += u64x4::splat((size << 32) + size);
            rotate_32_by(size, &mut self.v1);
            self.write(bytes_as_packet(self::remainder(remainder)));
        }
    }

    pub fn finish_u64(mut self, remainder: &[u8]) -> u64 {
        self.finish(remainder);
        for _ in 0..4 {
            self.write(permute(self.v0).to_array());
        }
        self.v0[0]
            .wrapping_add(self.v1[0])
            .wrapping_add(self.mul0[0])
            .wrapping_add(self.mul1[0])
    }
}

pub fn hash_u64(bytes: &[u8], key: [u64; 4]) -> u64 {
    let mut hasher = AutobahnHash::new(key);
    let (bytes, remainder) = bytes.split_at(bytes.len() / 32 * 32);
    for packet in bytes.chunks(32) {
        hasher.write(bytes_as_packet(packet.try_into().unwrap()));
    }
    hasher.finish_u64(remainder)
}
