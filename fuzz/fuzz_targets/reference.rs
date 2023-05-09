#![no_main]

use libfuzzer_sys::{arbitrary, fuzz_target};

extern "C" {
    fn HighwayHash64(data: *const u8, size: usize, key: *const u64) -> u64;
}

fn reference(data: &[u8], key: [u64; 4]) -> u64 {
    unsafe { HighwayHash64(data.as_ptr(), data.len(), key.as_ptr()) }
}

#[derive(Debug, arbitrary::Arbitrary)]
pub struct Input {
    pub key: [u64; 4],
    pub data: Vec<u8>,
}

fuzz_target!(|input: Input| {
    assert_eq!(
        reference(&input.data, input.key),
        autobahn_hash::hash_u64(&input.data, input.key)
    );
});
