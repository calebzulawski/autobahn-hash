#![feature(portable_simd)]
#![cfg_attr(not(feature = "multiversion"), no_std)]
#![deny(unsafe_code)]

use core::simd::{simd_swizzle, u32x8, u64x4, u8x32};

/// A hash instance.
#[derive(Clone, Debug)]
pub struct AutobahnHasher {
    v0: u64x4,
    v1: u64x4,
    mul0: u64x4,
    mul1: u64x4,
}

impl Default for AutobahnHasher {
    fn default() -> Self {
        Self::new()
    }
}

fn zipper_merge(x: u64x4) -> u64x4 {
    const INDEX: [usize; 32] = {
        let half_index = [3, 12, 2, 5, 14, 1, 15, 0, 11, 4, 10, 13, 9, 6, 8, 7];
        let mut index = [0; 32];
        let mut i = 0;
        while i < 32 {
            index[i] = i / 16 * 16 + half_index[i % 16];
            i += 1;
        }
        index
    };

    let x: u8x32 = bytemuck::cast(x);
    bytemuck::cast(simd_swizzle!(x, INDEX))
}

fn permute(x: u64x4) -> u64x4 {
    let x: u32x8 = bytemuck::cast(x);
    let x = simd_swizzle!(x, [5, 4, 7, 6, 1, 0, 3, 2]);
    bytemuck::cast(x)
}

fn remainder(bytes: &[u8]) -> [u8; 32] {
    let mut packet: [u8; 32] = [0u8; 32];
    let size_mod4 = bytes.len() & 3;
    let remaining = bytes.len() & !3;
    let size = bytes.len() as u64;

    packet[..remaining].copy_from_slice(&bytes[..remaining]);
    if size & 16 != 0 {
        packet[28..32].copy_from_slice(&bytes[remaining + size_mod4 - 4..remaining + size_mod4]);
    } else if size_mod4 != 0 {
        let remainder = &bytes[remaining..];
        packet[16] = remainder[0];
        packet[16 + 1] = remainder[size_mod4 >> 1];
        packet[16 + 2] = remainder[size_mod4 - 1];
    }

    packet
}

impl AutobahnHasher {
    /// Create a new `AutobahnHasher`.
    pub fn new() -> Self {
        Self::new_with_key([0; 4])
    }

    /// Create a new `AutobahnHasher` with the given key.
    pub fn new_with_key(key: [u64; 4]) -> Self {
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

    /// Write a packet of data to the hasher.
    pub fn write_packet(&mut self, packet: [u64; 4]) {
        let packet = u64x4::from_array(packet);
        self.v1 += self.mul0 + packet;
        self.mul0 ^= (self.v1 & u64x4::splat(0xffff_ffff)) * (self.v0 >> u64x4::splat(32));
        self.v0 += self.mul1;
        self.mul1 ^= (self.v0 & u64x4::splat(0xffff_ffff)) * (self.v1 >> u64x4::splat(32));
        self.v0 += zipper_merge(self.v1);
        self.v1 += zipper_merge(self.v0);
    }

    /// Write a packet of data to the hasher, in the form of bytes.
    pub fn write_bytes(&mut self, bytes: [u8; 32]) {
        let mut packet = [0; 4];
        for (i, chunk) in bytes.chunks(8).enumerate() {
            packet[i] = u64::from_le_bytes(chunk.try_into().unwrap());
        }
        self.write_packet(packet);
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
            self.write_bytes(self::remainder(remainder));
        }
    }

    /// Produce a `u64` hash.
    ///
    /// The `remainder` bytes must be less than a packet (less than 32 bytes).
    ///
    /// Writing the remainder is notably different than `Hasher::write`.  The remainder is padded
    /// and permuted into a 32-bit packet.
    pub fn finish_u64(mut self, remainder: &[u8]) -> u64 {
        self.finish(remainder);
        for _ in 0..4 {
            self.write_packet(permute(self.v0).to_array());
        }
        self.v0[0]
            .wrapping_add(self.v1[0])
            .wrapping_add(self.mul0[0])
            .wrapping_add(self.mul1[0])
    }
}

impl core::hash::Hasher for AutobahnHasher {
    fn finish(&self) -> u64 {
        self.clone().finish_u64(&[])
    }

    fn write(&mut self, bytes: &[u8]) {
        // `Hasher` requires calls to be exactly sequenced (e.g. two calls to `write` does not need
        // to be the same as a single call two `write` with the same data concatenated).
        // Therefore, we don't need to buffer and can simply pad bytes.
        let (bytes, remainder) = bytes.split_at(bytes.len() / 32 * 32);
        for packet in bytes.chunks(32) {
            self.write_bytes(packet.try_into().unwrap())
        }
        let mut packet = [0; 32];
        packet[..remainder.len()].copy_from_slice(remainder);
        self.write_bytes(packet);
    }
}

/// Hash a slice with the given key.
///
/// This function automatically dispatches to the optimal instruction set.
#[cfg_attr(feature = "multiversion", multiversion::multiversion(targets = "simd"))]
pub fn hash_u64(bytes: &[u8], key: [u64; 4]) -> u64 {
    let mut hasher = AutobahnHasher::new_with_key(key);
    let (bytes, remainder) = bytes.split_at(bytes.len() / 32 * 32);
    for packet in bytes.chunks(32) {
        hasher.write_bytes(packet.try_into().unwrap());
    }
    hasher.finish_u64(remainder)
}
