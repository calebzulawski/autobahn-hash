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
