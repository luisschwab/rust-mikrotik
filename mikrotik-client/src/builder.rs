//! Connection configuration for binary `RouterOS` API sessions.

use std::time::Duration;

use mikrotik_types::target::Credentials;

/// Default plaintext `RouterOS` API port.
pub const API_PORT: u16 = 8728;

/// Default TLS `RouterOS` API port.
pub const API_SSL_PORT: u16 = 8729;

/// Default SSH service port.
pub const SSH_PORT: u16 = 22;

/// Default Telnet service port.
pub const TELNET_PORT: u16 = 23;

/// Default FTP service port.
pub const FTP_PORT: u16 = 21;

/// Default HTTP service port.
pub const HTTP_PORT: u16 = 80;

/// Default HTTPS service port.
pub const HTTPS_PORT: u16 = 443;

/// Default `WinBox` service port.
pub const WINBOX_PORT: u16 = 8291;

/// Default MAC-Telnet service port.
pub const MAC_TELNET_PORT: u16 = 20561;

/// `RouterOS` management service protocol.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Protocol {
    /// Plaintext `RouterOS` API service.
    #[default]
    Api,
    /// TLS `RouterOS` API service.
    ApiSsl,
    /// SSH terminal service.
    Ssh,
    /// Telnet terminal service.
    Telnet,
    /// FTP file service.
    Ftp,
    /// Plaintext HTTP web service.
    Http,
    /// TLS HTTP web service.
    Https,
    /// `WinBox` management service.
    WinBox,
    /// Layer-2 MAC-Telnet service.
    MacTelnet,
}

impl Protocol {
    /// Return the default TCP port for this protocol.
    pub const fn default_port(self) -> u16 {
        match self {
            Self::Api => API_PORT,
            Self::ApiSsl => API_SSL_PORT,
            Self::Ssh => SSH_PORT,
            Self::Telnet => TELNET_PORT,
            Self::Ftp => FTP_PORT,
            Self::Http => HTTP_PORT,
            Self::Https => HTTPS_PORT,
            Self::WinBox => WINBOX_PORT,
            Self::MacTelnet => MAC_TELNET_PORT,
        }
    }
}

/// Connection configuration for one `RouterOS` device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientBuilder {
    /// Device host name or IP address without a port.
    pub host: String,
    /// Device service port.
    pub port: u16,
    /// `RouterOS` management service protocol.
    pub protocol: Protocol,
    /// Credentials used during the `RouterOS` login handshake.
    pub credentials: Credentials,
    /// Optional human-readable label used in client log events.
    pub log_label: Option<String>,
    /// Optional maximum time spent retrying transient connection failures.
    pub connect_retry_timeout: Option<Duration>,
    /// Optional maximum time spent in one TCP/login attempt.
    pub connect_attempt_timeout: Option<Duration>,
    /// Optional maximum delay between transient connection retries.
    pub connect_retry_max_delay: Option<Duration>,
}

impl ClientBuilder {
    /// Build client configuration using the protocol's default port.
    pub fn new(host: impl Into<String>, protocol: Protocol, credentials: Credentials) -> Self {
        Self {
            host: host.into(),
            port: protocol.default_port(),
            protocol,
            credentials,
            log_label: None,
            connect_retry_timeout: None,
            connect_attempt_timeout: None,
            connect_retry_max_delay: None,
        }
    }

    /// Override the service port.
    #[must_use]
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Add a human-readable label to client log events.
    #[must_use]
    pub fn with_log_label(mut self, label: impl Into<String>) -> Self {
        self.log_label = Some(label.into());
        self
    }

    /// Override the maximum time spent retrying transient connection failures.
    #[must_use]
    pub fn with_connect_retry_timeout(mut self, timeout: Duration) -> Self {
        self.connect_retry_timeout = Some(timeout);
        self
    }

    /// Override the maximum time spent in one TCP/login attempt.
    #[must_use]
    pub fn with_connect_attempt_timeout(mut self, timeout: Duration) -> Self {
        self.connect_attempt_timeout = Some(timeout);
        self
    }

    /// Override the maximum delay between transient connection retries.
    #[must_use]
    pub fn with_connect_retry_max_delay(mut self, delay: Duration) -> Self {
        self.connect_retry_max_delay = Some(delay);
        self
    }

    /// Return the maximum time spent retrying transient connection failures.
    pub(crate) fn connect_retry_timeout(&self, default: Duration) -> Duration {
        self.connect_retry_timeout.unwrap_or(default)
    }

    /// Return the maximum time spent in one TCP/login attempt.
    pub(crate) fn connect_attempt_timeout(&self, default: Duration) -> Duration {
        self.connect_attempt_timeout.unwrap_or(default)
    }

    /// Return the maximum delay between transient connection retries.
    pub(crate) fn connect_retry_max_delay(&self, default: Duration) -> Duration {
        self.connect_retry_max_delay.unwrap_or(default)
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
        assert_eq!(Protocol::Ssh.default_port(), SSH_PORT);
        assert_eq!(Protocol::Telnet.default_port(), TELNET_PORT);
        assert_eq!(Protocol::Ftp.default_port(), FTP_PORT);
        assert_eq!(Protocol::Http.default_port(), HTTP_PORT);
        assert_eq!(Protocol::Https.default_port(), HTTPS_PORT);
        assert_eq!(Protocol::WinBox.default_port(), WINBOX_PORT);
        assert_eq!(Protocol::MacTelnet.default_port(), MAC_TELNET_PORT);
    }

    #[test]
    fn builder_builds_socket_address() {
        let config = ClientBuilder::new("192.0.2.1", Protocol::Api, credentials());
        assert_eq!(config.socket_address(), "192.0.2.1:8728");

        let config = ClientBuilder::new("2001:db8::1", Protocol::ApiSsl, credentials());
        assert_eq!(config.socket_address(), "[2001:db8::1]:8729");

        let config = ClientBuilder::new("router.local", Protocol::Api, credentials()).with_port(18728);
        assert_eq!(config.socket_address(), "router.local:18728");
    }

    #[test]
    fn builder_accepts_optional_log_label_and_connect_timeouts() {
        let config = ClientBuilder::new("192.0.2.1", Protocol::Api, credentials())
            .with_log_label("R1")
            .with_connect_retry_timeout(Duration::from_secs(300))
            .with_connect_attempt_timeout(Duration::from_secs(3))
            .with_connect_retry_max_delay(Duration::from_secs(2));

        assert_eq!(config.log_label.as_deref(), Some("R1"));
        assert_eq!(
            config.connect_retry_timeout(Duration::from_secs(120)),
            Duration::from_secs(300)
        );
        assert_eq!(
            config.connect_attempt_timeout(Duration::from_secs(5)),
            Duration::from_secs(3)
        );
        assert_eq!(
            config.connect_retry_max_delay(Duration::from_secs(5)),
            Duration::from_secs(2)
        );
    }
}
