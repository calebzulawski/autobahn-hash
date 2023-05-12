#![no_main]

use libfuzzer_sys::{arbitrary, fuzz_target};

extern "C" {
    fn HighwayHash64(data: *const u8, size: usize, key: *const u64) -> u64;
    fn HighwayHash128(data: *const u8, size: usize, key: *const u64, output: *mut u64);
}

fn reference_64(data: &[u8], key: [u64; 4]) -> u64 {
    unsafe { HighwayHash64(data.as_ptr(), data.len(), key.as_ptr()) }
}

fn reference_128(data: &[u8], key: [u64; 4]) -> [u64; 2] {
    let mut out = [0; 2];
    unsafe { HighwayHash128(data.as_ptr(), data.len(), key.as_ptr(), out.as_mut_ptr()) }
    out
}

#[derive(Debug, arbitrary::Arbitrary)]
pub struct Input {
    pub key: [u64; 4],
    pub data: Vec<u8>,
}

fuzz_target!(|input: Input| {
    assert_eq!(
        reference_64(&input.data, input.key),
        autobahn_hash::hash_64(&input.data, input.key)
    );
    assert_eq!(
        reference_128(&input.data, input.key),
        autobahn_hash::hash_128(&input.data, input.key)
    );
});
