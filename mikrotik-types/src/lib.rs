#![no_std]

//! Versionless data models for `MikroTik`'s `RouterOS`.
//!
//! `RouterOS` API responses are maps of string properties. This crate provides
//! typed row structs and reusable primitive value types for deserializing those
//! maps while preserving the API's version and configuration variability with
//! optional fields.
//!
//! Response rows live under [`api`]. Reusable scalar and enum values live under
//! [`primitives`]. Higher-level observer/domain models, such as device
//! snapshots and topology, live outside both so they can compose API rows
//! without becoming wire-format types themselves.
//!
//! Endpoint row structs follow the `RouterOS` menu path in `PascalCase`,
//! dropping repeated namespace words only when the [`api`] submodule already
//! supplies that context. For example,
//! `/interface/ethernet/switch/port/print` maps to
//! [`api::interface::EthernetSwitchPort`], `/ip/settings/print` maps to
//! [`api::ip::IpSettings`], and `/routing/stats/origin/print` maps to
//! [`api::routing::RoutingStatsOrigin`].

extern crate alloc;

pub mod api;
pub mod device;
pub mod primitives;
pub mod target;
pub mod topology;

pub use mikrotik_common::Row;
pub use mikrotik_common::comma_list;
pub use mikrotik_common::comma_list_from_str;
pub use mikrotik_common::deserialize;
pub use mikrotik_common::optional_bool;
pub use mikrotik_common::optional_from_str;
pub use primitives::ParseError;
pub use primitives::RouterOsId;
pub(crate) use primitives::parse_non_empty;
