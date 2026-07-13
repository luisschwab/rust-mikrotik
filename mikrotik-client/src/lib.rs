//! # `MikroTik` Client
//!
//! `RouterOS` API client for interfacing with `MikroTik` devices.
//!
//! ## Protocol Support
//!
//! This client has support for the following protocols:
//! - [x] API
//! - [x] API SSL
//! - [ ] SSH
//! - [ ] Telnet
//! - [ ] MAC Telnet
//! - [ ] FTP
//! - [ ] HTTP
//! - [ ] HTTPS
//! - [ ] `WinBox`
//!
//! This crate uses the sans-IO `mikrotik-proto2` protocol implementation with a
//! Tokio transport, then exposes raw command execution and typed print commands
//! that deserialize into `mikrotik-types` endpoint rows.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use mikrotik_client::builder::ClientBuilder;
//! use mikrotik_client::builder::Protocol;
//! use mikrotik_client::client::Client;
//! use mikrotik_client::commands;
//! use mikrotik_client::types::target::Credentials;
//!
//! # async fn example() -> mikrotik_client::error::Result<()> {
//! let client = Client::connect(ClientBuilder::new(
//!     "10.21.21.1",
//!     Protocol::Api,
//!     Credentials {
//!         username: "admin".to_owned(),
//!         password: Some("password".to_owned()),
//!     },
//! ))
//! .await?;
//!
//! let identity = client.call("/system/identity/print", &[]).await?;
//! println!("identity rows: {identity:?}");
//!
//! let interfaces = client
//!     .print::<mikrotik_client::types::api::interface::Interface>(
//!         commands::PrintCommand::Interface(commands::Interface::Interface),
//!     )
//!     .await?;
//! println!("interfaces: {}", interfaces.len());
//! # Ok(())
//! # }
//! ```

pub mod builder;
pub mod client;
pub mod commands;
pub mod error;
pub mod print;
mod transport;

pub use mikrotik_types as types;
