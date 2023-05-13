#![feature(portable_simd)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

use core::simd::{simd_swizzle, u32x8, u64x4, u8x32};

/// A hash instance.
///
/// For maximum performance, use this hasher in a larger code block compiled with SIMD target
/// features enabled.
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

fn modular_reduction(a3_unmasked: u64, a2: u64, a1: u64, a0: u64) -> (u64, u64) {
    let a3 = a3_unmasked & 0x3fffffffffffffff;
    (
        a1 ^ ((a3 << 1) | (a2 >> 63)) ^ ((a3 << 2) | (a2 >> 62)),
        a0 ^ (a2 << 1) ^ (a2 << 2),
    )
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

    fn write_simd(&mut self, packet: u64x4) {
        self.v1 += self.mul0 + packet;
        self.mul0 ^= (self.v1 & u64x4::splat(0xffff_ffff)) * (self.v0 >> u64x4::splat(32));
        self.v0 += self.mul1;
        self.mul1 ^= (self.v0 & u64x4::splat(0xffff_ffff)) * (self.v1 >> u64x4::splat(32));
        self.v0 += zipper_merge(self.v1);
        self.v1 += zipper_merge(self.v0);
    }

    /// Write a packet of data to the hasher.
    pub fn write_packet(&mut self, packet: [u64; 4]) {
        let packet = u64x4::from_array(packet);
        self.write_simd(packet);
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

    /// Produce a 64-bit hash.
    ///
    /// The `remainder` bytes must be less than a packet (less than 32 bytes).
    ///
    /// Writing the remainder is notably different than `Hasher::write`.  The remainder is padded
    /// and permuted into a 32-bit packet.
    pub fn finish_64(mut self, remainder: &[u8]) -> u64 {
        self.finish(remainder);
        for _ in 0..4 {
            self.write_packet(permute(self.v0).to_array());
        }
        self.v0[0]
            .wrapping_add(self.v1[0])
            .wrapping_add(self.mul0[0])
            .wrapping_add(self.mul1[0])
    }

    /// Produce a 128-bit hash.
    ///
    /// The `remainder` bytes must be less than a packet (less than 32 bytes).
    ///
    /// Writing the remainder is notably different than `Hasher::write`.  The remainder is padded
    /// and permuted into a 32-bit packet.
    pub fn finish_128(mut self, remainder: &[u8]) -> [u64; 2] {
        self.finish(remainder);
        for _ in 0..6 {
            self.write_packet(permute(self.v0).to_array());
        }

        [
            self.v0[0]
                .wrapping_add(self.mul0[0])
                .wrapping_add(self.v1[2])
                .wrapping_add(self.mul1[2]),
            self.v0[1]
                .wrapping_add(self.mul0[1])
                .wrapping_add(self.v1[3])
                .wrapping_add(self.mul1[3]),
        ]
    }

    /// Produce a 256-bit hash.
    ///
    /// The `remainder` bytes must be less than a packet (less than 32 bytes).
    ///
    /// Writing the remainder is notably different than `Hasher::write`.  The remainder is padded
    /// and permuted into a 32-bit packet.
    pub fn finish_256(mut self, remainder: &[u8]) -> [u64; 4] {
        self.finish(remainder);
        for _ in 0..10 {
            self.write_packet(permute(self.v0).to_array());
        }

        let (m1, m0) = modular_reduction(
            self.v1[1].wrapping_add(self.mul1[1]),
            self.v1[0].wrapping_add(self.mul1[0]),
            self.v0[1].wrapping_add(self.mul0[1]),
            self.v0[0].wrapping_add(self.mul0[0]),
        );
        let (m3, m2) = modular_reduction(
            self.v1[3].wrapping_add(self.mul1[3]),
            self.v1[2].wrapping_add(self.mul1[2]),
            self.v0[3].wrapping_add(self.mul0[3]),
            self.v0[2].wrapping_add(self.mul0[2]),
        );
        [m0, m1, m2, m3]
    }
}

impl core::hash::Hasher for AutobahnHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.clone().finish_64(&[])
    }

    #[inline]
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

    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.write_u64(i as u64);
    }

    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.write_u64(i as u64);
    }

    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.write_u64(i as u64);
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.write_simd(u64x4::splat(i));
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        if core::mem::size_of::<usize>() > 8 {
            self.write(&i.to_ne_bytes());
        } else {
            self.write_u64(i as u64);
        }
    }
}

/// Hash a slice with the given key.
///
/// This function dynamically selects the best instruction set at runtime.
#[cfg(feature = "multiversion")]
#[cfg_attr(docsrs, doc(cfg(feature = "multiversion")))]
#[multiversion::multiversion(targets = "simd")]
pub fn hash_64(bytes: &[u8], key: [u64; 4]) -> u64 {
    let mut hasher = AutobahnHasher::new_with_key(key);
    let (bytes, remainder) = bytes.split_at(bytes.len() / 32 * 32);
    for packet in bytes.chunks(32) {
        hasher.write_bytes(packet.try_into().unwrap());
    }
    hasher.finish_64(remainder)
}

/// Hash a slice with the given key.
///
/// This function dynamically selects the best instruction set at runtime.
#[cfg(feature = "multiversion")]
#[cfg_attr(docsrs, doc(cfg(feature = "multiversion")))]
#[multiversion::multiversion(targets = "simd")]
pub fn hash_128(bytes: &[u8], key: [u64; 4]) -> [u64; 2] {
    let mut hasher = AutobahnHasher::new_with_key(key);
    let (bytes, remainder) = bytes.split_at(bytes.len() / 32 * 32);
    for packet in bytes.chunks(32) {
        hasher.write_bytes(packet.try_into().unwrap());
    }
    hasher.finish_128(remainder)
}

/// Hash a slice with the given key.
///
/// This function dynamically selects the best instruction set at runtime.
#[cfg(feature = "multiversion")]
#[cfg_attr(docsrs, doc(cfg(feature = "multiversion")))]
#[multiversion::multiversion(targets = "simd")]
pub fn hash_256(bytes: &[u8], key: [u64; 4]) -> [u64; 4] {
    let mut hasher = AutobahnHasher::new_with_key(key);
    let (bytes, remainder) = bytes.split_at(bytes.len() / 32 * 32);
    for packet in bytes.chunks(32) {
        hasher.write_bytes(packet.try_into().unwrap());
    }
    hasher.finish_256(remainder)
}
