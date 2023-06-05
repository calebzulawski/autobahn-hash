AutobahnHash
============
[![Crates.io](https://img.shields.io/crates/v/autobahn-hash)](https://crates.io/crates/autobahn-hash)
[![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://docs.rs/autobahn-hash)
[![License](https://img.shields.io/crates/l/autobahn-hash)](https://crates.io/crates/autobahn-hash)

A pure Rust implementation of [HighwayHash](https://github.com/google/highwayhash).

A few highlights:
* No `unsafe`
* Fuzzed against the reference implementation
* Minimal crate with few required dependencies
* Portable to any SIMD instruction set (and reasonably fast without SIMD)

This crate requires the `portable_simd` nightly feature.

## Benchmarks
The following benchmarks were done on an Intel i7-9750H, to give an idea of the performance profile.
These two benchmarks can help predict best- and worst-case performance.

### Slices
The HighwayHash algorithm performs best on long slices of data:
![slice benchmark](assets/slice.png)

### Non-slice data
Worst-case performance is can be predicted with non-slice data: `struct Data(u8, u16, u32, u64);`
![struct benchmark](assets/struct.png)

## License
AutobahnHash is distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.
