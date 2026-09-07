# mikrotik-meta

[![docs.rs][docs-rs-badge]][docs-rs-url]
[![license-mit-apache][license-badge]][license-url]

[docs-rs-badge]: https://img.shields.io/badge/docs.rs-mikrotik--meta-blue
[docs-rs-url]: https://docs.rs/mikrotik-meta
[license-badge]: https://img.shields.io/badge/License-MIT%2FApache--2.0-red.svg
[license-url]: https://github.com/luisschwab/rust-mikrotik/blob/master/LICENSE-MIT

Meta-crate for [`rust-mikrotik`](https://github.com/luisschwab/rust-mikrotik)'s crates.

This crate has no default features. Enable only the components needed by your application:

## Re-exported crates

| Crate | Feature | Re-export |
|-------|---------|-----------|
| [`mikrotik-client`] | `client` | `mikrotik_meta::client` |
| [`mikrotik-common`] | `common` | `mikrotik_meta::common` |
| [`mikrotik-crawler`] | `crawler` | `mikrotik_meta::crawler` |
| [`mikrotik-graphviz`] | `graphviz` | `mikrotik_meta::graphviz` |
| [`mikrotik-proto2`] | `proto2` | `mikrotik_meta::proto2` |
| [`mikrotik-qemu-runner`] | `qemu-runner` | `mikrotik_meta::qemu_runner` |
| [`mikrotik-types`] | `types` | `mikrotik_meta::types` |

Each dependency has its default features disabled. A feature controls
whether that crate is directly re-exported.

## Forwarded features

| Feature | Enables |
|---------|---------|
| `common-log` | `mikrotik-common/log` |
| `proto2-std` | `mikrotik-proto2/std` |

This crate is `#![no_std]` when no features are enabled. Individual features may
select crates that require `std`.

## Minimum Supported Rust Version

This set of libraries should compile with any combination of features on 
[Rust 1.85.0](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/).

To build with the MSRV toolchain, copy `Cargo-minimal.lock` to `Cargo.lock`.

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)
at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
