//! Connection configuration for binary `RouterOS` API sessions.

use mikrotik_types::target::Credentials;

/// Default plaintext `RouterOS` API port.
pub const API_PORT: u16 = 8728;

/// Default TLS `RouterOS` API port.
pub const API_SSL_PORT: u16 = 8729;

/// `RouterOS` binary API transport protocol.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Protocol {
    /// Plaintext `RouterOS` API service.
    #[default]
    Api,
    /// TLS `RouterOS` API service.
    ApiSsl,
}

impl Protocol {
    /// Return the default TCP port for this protocol.
    pub const fn default_port(self) -> u16 {
        match self {
            Self::Api => API_PORT,
            Self::ApiSsl => API_SSL_PORT,
        }
    }
}

/// Connection configuration for one `RouterOS` device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MikroTikClientConfig {
    /// Device host name or IP address without a port.
    pub host: String,
    /// Device `RouterOS` API port.
    pub port: u16,
    /// `RouterOS` API transport protocol.
    pub protocol: Protocol,
    /// Credentials used during the `RouterOS` login handshake.
    pub credentials: Credentials,
}

impl MikroTikClientConfig {
    /// Build client configuration using the protocol's default port.
    pub fn new(host: impl Into<String>, protocol: Protocol, credentials: Credentials) -> Self {
        Self {
            host: host.into(),
            port: protocol.default_port(),
            protocol,
            credentials,
        }
    }

    /// Override the `RouterOS` API port.
    #[must_use]
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Return the socket address string passed to the lower-level client.
    pub fn socket_address(&self) -> String {
        if self.host.contains(':') && !self.host.starts_with('[') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credentials() -> Credentials {
        Credentials {
            username: "admin".to_owned(),
            password: Some("password".to_owned()),
        }
    }

    #[test]
    fn protocol_default_ports_match_routeros_services() {
        assert_eq!(Protocol::Api.default_port(), API_PORT);
        assert_eq!(Protocol::ApiSsl.default_port(), API_SSL_PORT);
    }

    #[test]
    fn config_builds_socket_address() {
        let config = MikroTikClientConfig::new("192.0.2.1", Protocol::Api, credentials());
        assert_eq!(config.socket_address(), "192.0.2.1:8728");

        let config = MikroTikClientConfig::new("2001:db8::1", Protocol::ApiSsl, credentials());
        assert_eq!(config.socket_address(), "[2001:db8::1]:8729");

        let config = MikroTikClientConfig::new("router.local", Protocol::Api, credentials()).with_port(18728);
        assert_eq!(config.socket_address(), "router.local:18728");
    }
}
