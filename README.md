<div align="center">

<h1>Rust MikroTik</h1>

<p>
A set of libraries with support for de/serialization, parsing, executing
on data-structures and network messages, and interacting with MikroTik devices.
</p>

<p>
<a href="https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/">
    <img src="https://img.shields.io/badge/rustc-1.85.0%2B-orange.svg?label=MSRV"/>
</a>
<a href="https://github.com/luisschwab/rust-mikrotik/blob/master/LICENSE">
    <img src="https://img.shields.io/badge/License-MIT%2FApache--2.0-red.svg"/>
</a>
<a href="https://github.com/luisschwab/rust-mikrotik/actions/workflows/rust.yml">
    <img src="https://github.com/luisschwab/rust-mikrotik/actions/workflows/rust.yml/badge.svg">
</a>
<a href="https://github.com/luisschwab/rust-mikrotik/actions/workflows/simnet.yml">
    <img src="https://github.com/luisschwab/rust-mikrotik/actions/workflows/simnet.yml/badge.svg">
</a>
</p>

</div>

## Crates

Name | Crate | Purpose
---|---|---
`mikrotik-client` | [`mikrotik-client`](https://github.com/luisschwab/rust-mikrotik/tree/master/mikrotik-client) | A client for MikroTik's API
`mikrotik-types` | [`mikrotik-types`](https://github.com/luisschwab/rust-mikrotik/tree/master/mikrotik-types) | A collection of models for MikroTik's API

## Minimum Supported Rust Version

This set of library should compile with any combination of features on Rust 1.85.0.

To build with the MSRV toolchain, copy `Cargo-minimal.lock` to `Cargo.lock`.

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
