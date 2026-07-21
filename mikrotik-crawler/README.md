# mikrotik-crawler

[![crates.io][crates-io-badge]][crates-io-url]
[![docs.rs][docs-rs-badge]][docs-rs-url]
[![license-mit-apache][license-badge]][license-url]

[crates-io-badge]: https://img.shields.io/crates/v/mikrotik-crawler.svg
[crates-io-url]: https://crates.io/crates/mikrotik-crawler
[docs-rs-badge]: https://img.shields.io/badge/docs.rs-mikrotik--crawler-blue
[docs-rs-url]: https://docs.rs/mikrotik-crawler
[license-badge]: https://img.shields.io/badge/License-MIT%2FApache--2.0-red.svg
[license-url]: https://github.com/luisschwab/rust-mikrotik/blob/master/LICENSE-MIT

Long-running read-only RouterOS discovery and snapshot collection.

This crate keeps a target registry in memory, refreshes `CollectedSnapshot` values
with Tokio background tasks, and emits snapshot events for observers.
