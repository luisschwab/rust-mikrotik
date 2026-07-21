# mikrotik-common

[![crates.io][crates-io-badge]][crates-io-url]
[![docs.rs][docs-rs-badge]][docs-rs-url]
[![license-mit-apache][license-badge]][license-url]

[crates-io-badge]: https://img.shields.io/crates/v/mikrotik-common.svg
[crates-io-url]: https://crates.io/crates/mikrotik-common
[docs-rs-badge]: https://img.shields.io/badge/docs.rs-mikrotik--common-blue
[docs-rs-url]: https://docs.rs/mikrotik-common
[license-badge]: https://img.shields.io/badge/License-MIT%2FApache--2.0-red.svg
[license-url]: https://github.com/luisschwab/rust-mikrotik/blob/master/LICENSE-MIT

Shared internals for the `rust-mikrotik` workspace.

This crate contains small building blocks used by more than one workspace
crate, such as raw RouterOS row handling and serde helpers for RouterOS string
fields. It stays independent from higher-level client and endpoint-model APIs.

## no_std

`mikrotik-common` is `no_std` with `alloc`.
