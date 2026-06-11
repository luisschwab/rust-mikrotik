#![deny(missing_docs)]

//! Binary `RouterOS` API client for `MikroTik` devices.
//!
//! This crate uses the sans-IO `mikrotik-proto` protocol implementation with a
//! Tokio transport, then exposes raw command execution and typed print methods
//! that deserialize into `mikrotik-types` endpoint rows.

mod client;
pub mod commands;
mod config;
mod error;
mod print;
pub mod print_checks;
mod transport;

pub use client::MikroTikClient;
pub use config::API_PORT;
pub use config::API_SSL_PORT;
pub use config::MikroTikClientConfig;
pub use config::Protocol;
pub use error::DecodeError;
pub use error::Error;
pub use error::Result;
pub use mikrotik_types as types;
pub use print::PrintMethods;
