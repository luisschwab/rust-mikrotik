# mikrotik-graphviz

[![crates.io][crates-io-badge]][crates-io-url]
[![docs.rs][docs-rs-badge]][docs-rs-url]
[![license-mit-apache][license-badge]][license-url]

[crates-io-badge]: https://img.shields.io/crates/v/mikrotik-graphviz.svg
[crates-io-url]: https://crates.io/crates/mikrotik-graphviz
[docs-rs-badge]: https://img.shields.io/badge/docs.rs-mikrotik--graphviz-blue
[docs-rs-url]: https://docs.rs/mikrotik-graphviz
[license-badge]: https://img.shields.io/badge/License-MIT%2FApache--2.0-red.svg
[license-url]: https://github.com/luisschwab/rust-mikrotik/blob/master/LICENSE-MIT

Graph construction and export for MikroTik device snapshots.

This crate builds topology graphs from `GraphSnapshot` values and exports DOT,
SVG/PNG via Graphviz, and interactive HTML artifacts.
