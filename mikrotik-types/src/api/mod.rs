//! Typed `RouterOS` API response rows.
//!
//! Modules mirror `RouterOS` menu families. Row structs keep fields optional so
//! one versionless type can represent rows across `RouterOS` patch/minor releases
//! and device-specific configuration.

pub mod interface;
pub mod ip;
pub mod queue;
pub mod routing;
pub mod service;
pub mod snmp;
pub mod system;
pub mod tool;
pub mod user;
