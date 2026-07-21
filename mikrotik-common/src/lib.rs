#![no_std]

//! Shared helpers for `rust-mikrotik` crates.
//!
//! This crate contains small, stable building blocks that are needed by more
//! than one package, without tying those packages to higher-level client or
//! endpoint-model APIs.

extern crate alloc;

pub mod format;
pub mod logging;
pub mod parse;
pub mod redaction;
pub mod row;
pub mod serde;
