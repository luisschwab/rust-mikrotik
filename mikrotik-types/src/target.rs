//! Collection target and credential types.
//!
//! This module contains local observer configuration models rather than
//! `RouterOS` API response rows. It is kept in `mikrotik-types` so clients and
//! tools can share the same target representation without depending on a
//! collector implementation.

use alloc::borrow::ToOwned as _;
use alloc::string::String;
use core::fmt;

use serde::Deserialize;
use serde::Serialize;

/// Errors raised while constructing observer domain types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObserverError {
    /// A device address was empty or whitespace-only.
    EmptyAddress,
}

impl fmt::Display for ObserverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAddress => formatter.write_str("device address is empty"),
        }
    }
}

/// `RouterOS` API credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Credentials {
    /// `RouterOS` username.
    pub username: String,
    /// `RouterOS` password, if the account has one.
    pub password: Option<String>,
}

/// `RouterOS` device address plus credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceTarget {
    /// Address passed to the `RouterOS` client, usually `host:port`.
    pub address: String,
    /// Credentials used for this target.
    pub credentials: Credentials,
}

impl DeviceTarget {
    /// Build a device target from address, username, and optional password.
    ///
    /// # Errors
    ///
    /// Returns [`ObserverError::EmptyAddress`] if the address is empty after
    /// trimming whitespace.
    pub fn new(
        address: impl Into<String>,
        username: impl Into<String>,
        password: Option<String>,
    ) -> Result<Self, ObserverError> {
        let address = address.into().trim().to_owned();
        if address.is_empty() {
            return Err(ObserverError::EmptyAddress);
        }

        Ok(Self {
            address,
            credentials: Credentials {
                username: username.into(),
                password,
            },
        })
    }
}
