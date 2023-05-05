#![feature(portable_simd)]

use core::simd::{simd_swizzle, u32x8, u64x4, u8x32};

/// A hash instance.
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
}
