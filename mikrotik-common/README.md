# mikrotik-common

<p>
    <a href="https://crates.io/crates/mikrotik-common"><img src="https://img.shields.io/crates/v/mikrotik-common.svg"/></a>
    <a href="https://docs.rs/mikrotik-common"><img src="https://img.shields.io/badge/docs.rs-mikrotik--common-blue"/></a>
    <a href="https://github.com/luisschwab/rust-mikrotik/blob/master/LICENSE-MIT"><img src="https://img.shields.io/badge/License-MIT%2FApache--2.0-red.svg"/></a>
</p>

Shared internals for the `rust-mikrotik` workspace.

This crate contains small building blocks used by more than one workspace
crate, such as raw RouterOS row handling and serde helpers for RouterOS string
fields. It stays independent from higher-level client and endpoint-model APIs.

## no_std

`mikrotik-common` is `no_std` with `alloc`.
