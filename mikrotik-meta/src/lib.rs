//! # `mikrotik-meta`
//!
//! Meta-crate for [`rust-mikrotik`](https://github.com/luisschwab/rust-mikrotik)'s crates.
//!
//! This crate has no default features. Enable only the components needed by your application:
//!
//! ## Re-exported crates
//!
//! | Feature       | Crate                  | Re-export                    |
//! |---------------|------------------------|------------------------------|
//! | `client`      | `mikrotik-client`      | `mikrotik_meta::client`      |
//! | `common`      | `mikrotik-common`      | `mikrotik_meta::common`      |
//! | `crawler`     | `mikrotik-crawler`     | `mikrotik_meta::crawler`     |
//! | `graphviz`    | `mikrotik-graphviz`    | `mikrotik_meta::graphviz`    |
//! | `proto2`      | `mikrotik-proto2`      | `mikrotik_meta::proto2`      |
//! | `qemu-runner` | `mikrotik-qemu-runner` | `mikrotik_meta::qemu_runner` |
//! | `types`       | `mikrotik-types`       | `mikrotik_meta::types`       |
//!
//! Each dependency has its default features disabled. A feature controls
//! whether that crate is directly re-exported.
//!
//! ## Forwarded features
//!
//! | Feature      | Enables                   |
//! |--------------|---------------------------|
//! | `common-log` | `mikrotik-common/log`     |
//! | `proto2-std` | `mikrotik-proto2/std`     |
//!
//! This crate is `#![no_std]` when no features are enabled. Individual
//! features may select crates that require `std`.

#![no_std]

/// Tokio-backed client for the `RouterOS` binary API.
#[cfg(feature = "client")]
pub use mikrotik_client as client;
/// Shared utilities used across the `rust-mikrotik` workspace.
#[cfg(feature = "common")]
pub use mikrotik_common as common;
/// Network crawler and snapshot collector.
#[cfg(feature = "crawler")]
pub use mikrotik_crawler as crawler;
/// Graphviz constructor and exporter for device snapshots.
#[cfg(feature = "graphviz")]
pub use mikrotik_graphviz as graphviz;
/// Sans-IO implementation of the `RouterOS` binary API protocol.
#[cfg(feature = "proto2")]
pub use mikrotik_proto2 as proto2;
/// QEMU-powered `RouterOS` CHR simulation harness.
#[cfg(feature = "qemu-runner")]
pub use mikrotik_qemu_runner as qemu_runner;
/// Versionless `RouterOS` API and domain data models.
#[cfg(feature = "types")]
pub use mikrotik_types as types;
