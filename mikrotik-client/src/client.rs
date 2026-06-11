//! Connected client and raw command execution.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Once;

use mikrotik_proto::CommandBuilder;
use mikrotik_proto::Event;
use mikrotik_types::Row;
use serde::de::DeserializeOwned;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use crate::config::MikroTikClientConfig;
use crate::error::DecodeError;
use crate::error::Error;
use crate::error::Result;
use crate::transport::Session;

/// Connected `RouterOS` binary API client.
#[derive(Debug, Clone)]
pub struct MikroTikClient {
    /// Connection configuration used to create the session.
    config: MikroTikClientConfig,
    /// Shared serialized access to the underlying protocol session.
    session: Arc<Mutex<Session>>,
}

impl MikroTikClient {
    /// Connect to a `RouterOS` device and complete the login handshake.
    ///
    /// # Errors
    ///
    /// Returns an error if TCP/TLS connection setup or `RouterOS` authentication
    /// fails.
    pub async fn connect(config: MikroTikClientConfig) -> Result<Self> {
        install_rustls_provider();
        let session = Session::connect(&config).await?;
        Ok(Self {
            config,
            session: Arc::new(Mutex::new(session)),
        })
    }

    /// Return this client's connection configuration.
    pub fn config(&self) -> &MikroTikClientConfig {
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

    /// Execute a print command and deserialize every row into `T`.
    pub(crate) async fn print_typed<T>(&self, command: &str) -> Result<Vec<T>>
    where
        T: DeserializeOwned,
    {
        let rows = self.call(command, &[]).await?;
        let mut typed_rows = Vec::with_capacity(rows.len());

        for (row_index, row) in rows.iter().enumerate() {
            let typed_row = mikrotik_types::deserialize(row)
                .map_err(|error| Error::Decode(DecodeError::new(command, row_index, error.to_string(), row)))?;
            typed_rows.push(typed_row);
        }

        Ok(typed_rows)
    }
}

impl Session {
    /// Send one encoded command and collect reply rows for its tag.
    async fn call(&mut self, command: mikrotik_proto::Command) -> Result<Vec<Row>> {
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
fn row_from_attributes(attributes: mikrotik_proto::HashMap<String, Option<String>>) -> Row {
    attributes
        .into_iter()
        .filter_map(|(key, value)| value.map(|value| (key, value)))
        .collect::<BTreeMap<_, _>>()
}

/// Install the process-wide rustls crypto provider once.
fn install_rustls_provider() {
    static RUSTLS_PROVIDER: Once = Once::new();
    RUSTLS_PROVIDER.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_conversion_drops_none_values() {
        let attributes = mikrotik_proto::HashMap::from([
            ("dst-address".to_owned(), Some("0.0.0.0/0".to_owned())),
            ("comment".to_owned(), None),
        ]);

        let row = row_from_attributes(attributes);

        assert_eq!(row.get("dst-address").map(String::as_str), Some("0.0.0.0/0"));
        assert!(!row.contains_key("comment"));
    }
}
