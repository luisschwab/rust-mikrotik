//! Collection target and credential types.
//!
//! This module contains local observer configuration models rather than
//! `RouterOS` API response rows. It is kept in `mikrotik-types` so clients and
//! tools can share the same target representation without depending on a
//! collector implementation.

use alloc::string::String;
use core::fmt;
use core::net::IpAddr;
use core::net::SocketAddr;

use serde::Deserialize;
use serde::Serialize;

/// Errors raised while constructing observer domain types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObserverError {
    /// A device address was empty or whitespace-only.
    EmptyAddress,
    /// A device address was not an IP socket address.
    InvalidAddress,
}

impl fmt::Display for ObserverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAddress => f.write_str("device address is empty"),
            Self::InvalidAddress => f.write_str("device address must be an IP address or IP:port"),
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

/// Connection target for a `RouterOS` device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceTarget {
    /// Address passed to the `RouterOS` client.
    pub address: SocketAddr,
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
        let address = parse_device_socket_addr(&address.into())?;

        Ok(Self {
            address,
            credentials: Credentials {
                username: username.into(),
                password,
            },
        })
    }
}

/// Parse a device target address, defaulting to the `RouterOS` API port.
fn parse_device_socket_addr(address: &str) -> Result<SocketAddr, ObserverError> {
    let address = address.trim();
    if address.is_empty() {
        return Err(ObserverError::EmptyAddress);
    }
    if let Ok(address) = address.parse() {
        return Ok(address);
    }
    if let Ok(ip) = address.parse::<IpAddr>() {
        return Ok(SocketAddr::new(ip, 8728));
    }
    if let Some(rest) = address.strip_prefix('[') {
        let Some((host, rest)) = rest.split_once(']') else {
            return Err(ObserverError::InvalidAddress);
        };
        let Some(port) = rest.strip_prefix(':') else {
            return Err(ObserverError::InvalidAddress);
        };
        let ip = host.parse().map_err(|_| ObserverError::InvalidAddress)?;
        let port = port.parse().map_err(|_| ObserverError::InvalidAddress)?;
        return Ok(SocketAddr::new(ip, port));
    }
    let Some((host, port)) = address.rsplit_once(':') else {
        return Err(ObserverError::InvalidAddress);
    };
    let ip = host.parse().map_err(|_| ObserverError::InvalidAddress)?;
    let port = port.parse().map_err(|_| ObserverError::InvalidAddress)?;
    Ok(SocketAddr::new(ip, port))
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString as _;

    use super::*;

    #[test]
    fn device_target_defaults_routeros_api_port() {
        let target = DeviceTarget::new("192.0.2.1", "admin", None).unwrap();

        assert_eq!(target.address.to_string(), "192.0.2.1:8728");
    }

    #[test]
    fn device_target_rejects_hostnames() {
        let error = DeviceTarget::new("router.example:8728", "admin", None).unwrap_err();

        assert_eq!(error, ObserverError::InvalidAddress);
    }

    #[test]
    fn device_target_parses_ipv6_with_default_port() {
        let target = DeviceTarget::new("2001:db8::1", "admin", None).unwrap();

        assert_eq!(target.address.to_string(), "[2001:db8::1]:8728");
    }
}
