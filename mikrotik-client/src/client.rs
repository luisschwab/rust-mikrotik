//! Connected client and raw command execution.

use core::time::Duration;
use std::io;
use std::io::ErrorKind;
use std::sync::Arc;
use std::sync::Once;

use mikrotik_common::row::Row;
use mikrotik_proto2::CommandBuilder;
use mikrotik_proto2::Event;
use serde::de::DeserializeOwned;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tokio::time::Instant;
use tokio::time::sleep;
use tokio::time::timeout;
use tracing::debug;

use crate::builder::ClientBuilder;
use crate::commands::PrintCommand;
use crate::error::DecodeError;
use crate::error::Error;
use crate::error::Result;
use crate::transport::Session;

/// Default maximum time spent retrying transient connection failures.
const CONNECT_RETRY_TIMEOUT: Duration = Duration::from_secs(120);
/// Maximum time allowed for one TCP/login attempt before retry backoff.
const CONNECT_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(10);
/// First delay used after a transient connection failure.
const CONNECT_RETRY_INITIAL_DELAY: Duration = Duration::from_millis(250);
/// Maximum delay used by exponential connection backoff.
const CONNECT_RETRY_MAX_DELAY: Duration = Duration::from_secs(5);

/// Connected `RouterOS` binary API client.
#[derive(Debug, Clone)]
pub struct Client {
    /// Connection configuration used to create the session.
    config: ClientBuilder,
    /// Shared serialized access to the underlying protocol session.
    session: Arc<Mutex<Session>>,
}

impl Client {
    /// Connect to a `RouterOS` device and complete the login handshake.
    ///
    /// # Errors
    ///
    /// Returns an error if TCP/TLS connection setup or `RouterOS` authentication
    /// fails. Transient transport errors are retried with exponential backoff
    /// before the final error is returned.
    pub async fn connect(config: ClientBuilder) -> Result<Self> {
        install_rustls_provider();
        let deadline = Instant::now() + config.connect_retry_timeout(CONNECT_RETRY_TIMEOUT);
        let attempt_timeout = config.connect_attempt_timeout(CONNECT_ATTEMPT_TIMEOUT);
        let retry_max_delay = config.connect_retry_max_delay(CONNECT_RETRY_MAX_DELAY);
        let mut delay = CONNECT_RETRY_INITIAL_DELAY;
        let mut last_transient_error = None;

        let session = loop {
            let attempt_started = Instant::now();
            match connect_attempt(&config, deadline, attempt_timeout).await {
                Ok(session) => break session,
                Err(error) if is_transient_connect_error(&error) && Instant::now() < deadline => {
                    let attempt_elapsed = attempt_started.elapsed();
                    let sleep_for = delay.min(deadline.saturating_duration_since(Instant::now()));
                    if let Some(label) = &config.log_label {
                        debug!(
                            "{}: RouterOS API at socket={} not ready after {} seconds: {error}. Retrying in {:?}...",
                            label,
                            config.socket_address(),
                            attempt_elapsed.as_secs(),
                            sleep_for
                        );
                    } else {
                        debug!(
                            "RouterOS API at socket={} is not ready after {} seconds: {error}. Retrying in {:?}...",
                            config.socket_address(),
                            attempt_elapsed.as_secs(),
                            sleep_for
                        );
                    }
                    last_transient_error = Some(error);
                    sleep(sleep_for).await;
                    delay = next_connect_delay(delay, retry_max_delay);
                }
                Err(error) if is_transient_connect_error(&error) => {
                    return Err(last_transient_error.unwrap_or(error));
                }
                Err(error) => return Err(error),
            }
        };

        Ok(Self {
            config,
            session: Arc::new(Mutex::new(session)),
        })
    }

    /// Return this client's connection configuration.
    pub fn config(&self) -> &ClientBuilder {
        &self.config
    }

    /// Execute a raw `RouterOS` command and collect all reply rows.
    ///
    /// Attribute entries with `None` values are sent as flag attributes.
    ///
    /// # Errors
    ///
    /// Returns an error if the command cannot be sent, if `RouterOS` returns a
    /// trap or fatal response, or if the connection closes before completion.
    pub async fn call(&self, command: &str, attributes: &[(&str, Option<&str>)]) -> Result<Vec<Row>> {
        let mut command_builder = CommandBuilder::new().command(command);
        for (key, value) in attributes {
            command_builder = command_builder.attribute(key, *value);
        }

        let mut session = self.session.lock().await;
        let rows = session.call(command_builder.build()).await?;

        Ok(rows)
    }

    /// Execute a typed print command and deserialize every row into `T`.
    ///
    /// # Errors
    ///
    /// Returns an error if the command cannot be sent, if `RouterOS` returns a
    /// trap or fatal response, if the connection closes before completion, or
    /// if any row cannot be decoded into `T`.
    pub async fn print<T>(&self, command: PrintCommand) -> Result<Vec<T>>
    where
        T: DeserializeOwned,
    {
        self.print_typed(command.as_path()).await
    }

    /// Execute a print command and deserialize every row into `T`.
    pub(crate) async fn print_typed<T>(&self, command: &str) -> Result<Vec<T>>
    where
        T: DeserializeOwned,
    {
        let rows = self.call(command, &[]).await?;
        let mut typed_rows = Vec::with_capacity(rows.len());

        for (row_index, row) in rows.iter().enumerate() {
            let typed_row = mikrotik_common::serde::deserialize(row)
                .map_err(|error| Error::Decode(DecodeError::new(command, row_index, error.to_string(), row)))?;
            typed_rows.push(typed_row);
        }

        Ok(typed_rows)
    }
}

/// Run one connection attempt with a bounded TCP/login handshake duration.
async fn connect_attempt(config: &ClientBuilder, deadline: Instant, attempt_timeout: Duration) -> Result<Session> {
    let timeout_for = attempt_timeout.min(deadline.saturating_duration_since(Instant::now()));
    match timeout(timeout_for, Session::connect(config)).await {
        Ok(result) => result,
        Err(_) => Err(Error::Io(io::Error::new(
            ErrorKind::TimedOut,
            format!("connect attempt exceeded {attempt_timeout:?}"),
        ))),
    }
}

/// Return whether a connect error is likely caused by an API service that is not ready yet.
fn is_transient_connect_error(error: &Error) -> bool {
    match error {
        Error::Io(error) => matches!(
            error.kind(),
            ErrorKind::ConnectionRefused
                | ErrorKind::ConnectionReset
                | ErrorKind::ConnectionAborted
                | ErrorKind::NotConnected
                | ErrorKind::TimedOut
                | ErrorKind::WouldBlock
        ),
        Error::ConnectionClosed => true,
        Error::Connection(_)
        | Error::Login(_)
        | Error::UnsupportedProtocol(_)
        | Error::Trap(_)
        | Error::Fatal(_)
        | Error::Decode(_) => false,
    }
}

/// Return the next exponential connect retry delay.
fn next_connect_delay(delay: Duration, max_delay: Duration) -> Duration {
    delay.saturating_mul(2).min(max_delay)
}

impl Session {
    /// Send one encoded command and collect reply rows for its tag.
    async fn call(&mut self, command: mikrotik_proto2::Command) -> Result<Vec<Row>> {
        let tag = self.connection.send_command(command)?;
        let mut rows = Vec::new();

        self.flush_transmits().await?;

        loop {
            while let Some(event) = self.connection.poll_event() {
                match event {
                    Event::Reply {
                        tag: event_tag,
                        response,
                    } if event_tag == tag => rows.push(row_from_attributes(response.attributes)),
                    Event::Done { tag: event_tag } | Event::Empty { tag: event_tag } if event_tag == tag => {
                        return Ok(rows);
                    }
                    Event::Trap {
                        tag: event_tag,
                        response,
                    } if event_tag == tag => return Err(Error::Trap(response.message)),
                    Event::Fatal { reason } => return Err(Error::Fatal(reason)),
                    Event::Reply { .. } | Event::Done { .. } | Event::Empty { .. } | Event::Trap { .. } => {}
                }
            }

            let mut buffer = [0u8; 8192];
            let read = self.stream.read(&mut buffer).await?;
            if read == 0 {
                return Err(Error::ConnectionClosed);
            }

            self.connection.receive(&buffer[..read])?;
            self.flush_transmits().await?;
        }
    }

    /// Write all pending protocol transmissions to the transport stream.
    async fn flush_transmits(&mut self) -> Result<()> {
        while let Some(transmit) = self.connection.poll_transmit() {
            self.stream.write_all(&transmit.data).await?;
        }
        Ok(())
    }
}

/// Convert protocol attributes into a `Row`, dropping absent values.
fn row_from_attributes(attributes: mikrotik_proto2::HashMap<String, Option<String>>) -> Row {
    attributes
        .into_iter()
        .filter_map(|(key, value)| value.map(|value| (key, value)))
        .collect()
}

/// Install the process-wide `rustls` crypto provider used by `mikrotik-client`.
fn install_rustls_provider() {
    static RUSTLS_PROVIDER: Once = Once::new();
    RUSTLS_PROVIDER.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

#[cfg(test)]
mod tests {
    use std::io::Error as IoError;

    use super::*;

    #[test]
    fn row_conversion_drops_none_values() {
        let attributes = mikrotik_proto2::HashMap::from([
            ("dst-address".to_owned(), Some("0.0.0.0/0".to_owned())),
            ("comment".to_owned(), None),
        ]);

        let row = row_from_attributes(attributes);

        assert_eq!(row.get("dst-address").map(String::as_str), Some("0.0.0.0/0"));
        assert!(!row.contains_key("comment"));
    }

    #[test]
    fn connect_backoff_doubles_until_cap() {
        assert_eq!(
            next_connect_delay(CONNECT_RETRY_INITIAL_DELAY, CONNECT_RETRY_MAX_DELAY),
            Duration::from_millis(500)
        );
        assert_eq!(
            next_connect_delay(Duration::from_secs(4), CONNECT_RETRY_MAX_DELAY),
            CONNECT_RETRY_MAX_DELAY
        );
        assert_eq!(
            next_connect_delay(CONNECT_RETRY_MAX_DELAY, CONNECT_RETRY_MAX_DELAY),
            CONNECT_RETRY_MAX_DELAY
        );
        assert_eq!(
            next_connect_delay(Duration::from_secs(4), Duration::from_secs(1)),
            Duration::from_secs(1)
        );
    }

    #[test]
    fn connect_backoff_retries_only_transient_errors() {
        assert!(is_transient_connect_error(&Error::Io(IoError::from(
            ErrorKind::ConnectionRefused
        ))));
        assert!(is_transient_connect_error(&Error::Io(IoError::from(
            ErrorKind::TimedOut
        ))));
        assert!(is_transient_connect_error(&Error::ConnectionClosed));
        assert!(!is_transient_connect_error(&Error::Io(IoError::from(
            ErrorKind::PermissionDenied
        ))));
        assert!(!is_transient_connect_error(&Error::Trap("bad command".to_owned())));
    }
}
