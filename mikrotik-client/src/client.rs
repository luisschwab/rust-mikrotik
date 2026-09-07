//! Connected client and raw command execution.

use core::time::Duration;
use std::io::ErrorKind;
use std::sync::Arc;

use mikrotik_common::row::Row;
use mikrotik_common::serde::deserialize;
use mikrotik_proto2::Command;
use mikrotik_proto2::CommandBuilder;
use mikrotik_proto2::Event;
use mikrotik_proto2::HashMap;
use mikrotik_proto2::response::TrapResponse;
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
        let rows = session
            .call(command, command_builder.build())
            .await
            .map_err(|error| error.with_command(command))?;

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
        let command = command.as_path();
        let rows = self.call(command, &[]).await?;
        let mut typed_rows = Vec::with_capacity(rows.len());

        for (row_index, row) in rows.iter().enumerate() {
            let typed_row = deserialize(row)
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
        Err(_) => Err(Error::Timeout {
            operation: "RouterOS connection attempt",
            duration: attempt_timeout,
        }),
    }
}

/// Return whether a connect error is likely caused by an API service that is not ready yet.
fn is_transient_connect_error(error: &Error) -> bool {
    match error {
        Error::Transport { source, .. } => matches!(
            source.kind(),
            ErrorKind::ConnectionRefused
                | ErrorKind::ConnectionReset
                | ErrorKind::ConnectionAborted
                | ErrorKind::NotConnected
                | ErrorKind::TimedOut
                | ErrorKind::WouldBlock
        ),
        Error::ConnectionClosed { .. } | Error::Timeout { .. } => true,
        Error::Connection { .. }
        | Error::Login(_)
        | Error::UnsupportedProtocol(_)
        | Error::PermissionDenied { .. }
        | Error::UnsupportedCommand { .. }
        | Error::Trap { .. }
        | Error::Fatal { .. }
        | Error::Decode(_) => false,
    }
}

/// Return the next exponential connect retry delay.
fn next_connect_delay(delay: Duration, max_delay: Duration) -> Duration {
    delay.saturating_mul(2).min(max_delay)
}

impl Session {
    /// Send one encoded command and collect reply rows for its tag.
    async fn call(&mut self, command_path: &str, command: Command) -> Result<Vec<Row>> {
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
                    } if event_tag == tag => return Err(classify_trap(command_path, response)),
                    Event::Fatal { reason } => {
                        return Err(Error::Fatal {
                            command: command_path.to_owned(),
                            reason,
                        });
                    }
                    Event::Reply { .. } | Event::Done { .. } | Event::Empty { .. } | Event::Trap { .. } => {}
                }
            }

            let mut buffer = [0u8; 8192];
            let read = self.stream.read(&mut buffer).await?;
            if read == 0 {
                return Err(Error::ConnectionClosed {
                    command: Some(command_path.to_owned()),
                });
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

/// Convert a `RouterOS` trap into the most specific public client error.
fn classify_trap(command: &str, response: TrapResponse) -> Error {
    let message_lower = response.message.to_ascii_lowercase();
    if message_lower.contains("not enough permissions")
        || message_lower.contains("permission denied")
        || message_lower.contains("not permitted")
    {
        return Error::PermissionDenied {
            command: command.to_owned(),
            message: response.message,
        };
    }
    if message_lower.contains("no such command") || message_lower.contains("unknown command") {
        return Error::UnsupportedCommand {
            command: command.to_owned(),
            message: response.message,
        };
    }

    Error::Trap {
        command: command.to_owned(),
        category: response.category,
        message: response.message,
    }
}

/// Convert protocol attributes into a `Row`, dropping absent values.
fn row_from_attributes(attributes: HashMap<String, Option<String>>) -> Row {
    attributes
        .into_iter()
        .filter_map(|(key, value)| value.map(|value| (key, value)))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::io::Error as IoError;

    use mikrotik_proto2::Tag;
    use mikrotik_proto2::codec;
    use mikrotik_proto2::codec::Decode;
    use mikrotik_proto2::word::Word;
    use mikrotik_types::target::Credentials;
    use serde::Deserialize;
    use tokio::io::DuplexStream;

    use super::*;
    use crate::builder::Protocol;
    use crate::commands::system::System;

    fn config() -> ClientBuilder {
        ClientBuilder::new(
            "192.0.2.1",
            Protocol::Api,
            Credentials {
                username: "admin".to_owned(),
                password: None,
            },
        )
    }

    fn test_client(stream: DuplexStream) -> Client {
        Client {
            config: config(),
            session: Arc::new(Mutex::new(Session {
                stream: Box::new(stream),
                connection: mikrotik_proto2::Connection::new(),
            })),
        }
    }

    async fn read_command_tag(stream: &mut DuplexStream) -> Tag {
        let mut data = Vec::new();
        loop {
            let mut buffer = [0; 512];
            let read = stream.read(&mut buffer).await.unwrap();
            assert_ne!(read, 0, "client closed before sending a command");
            data.extend_from_slice(&buffer[..read]);

            let Decode::Complete { value: sentence, .. } = codec::decode_sentence(&data).unwrap() else {
                continue;
            };
            return sentence
                .typed_words()
                .find_map(|word| match word.unwrap() {
                    Word::Tag(tag) => Some(tag),
                    _ => None,
                })
                .expect("command should contain a tag");
        }
    }

    fn sentence(words: &[&[u8]]) -> Vec<u8> {
        let mut data = Vec::new();
        for word in words {
            codec::encode_word(word, &mut data);
        }
        codec::encode_terminator(&mut data);
        data
    }

    async fn send_reply_and_done(stream: &mut DuplexStream, attributes: &[&[u8]]) {
        let tag = read_command_tag(stream).await;
        let tag_word = format!(".tag={tag}");
        let mut reply_words = vec![b"!re".as_ref(), tag_word.as_bytes()];
        reply_words.extend_from_slice(attributes);

        let mut response = sentence(&reply_words);
        response.extend_from_slice(&sentence(&[b"!done", tag_word.as_bytes()]));
        stream.write_all(&response).await.unwrap();
    }

    #[test]
    fn row_conversion_drops_none_values() {
        let attributes = HashMap::from([
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
        assert!(is_transient_connect_error(&Error::Transport {
            command: None,
            source: IoError::from(ErrorKind::ConnectionRefused),
        }));
        assert!(is_transient_connect_error(&Error::Timeout {
            operation: "test connection",
            duration: Duration::from_secs(1),
        }));
        assert!(is_transient_connect_error(&Error::ConnectionClosed { command: None }));
        assert!(!is_transient_connect_error(&Error::Transport {
            command: None,
            source: IoError::from(ErrorKind::PermissionDenied),
        }));
        assert!(!is_transient_connect_error(&Error::Trap {
            command: "/test/print".to_owned(),
            category: None,
            message: "bad command".to_owned(),
        }));
    }

    #[test]
    fn traps_are_classified_without_losing_command_context() {
        let permission = classify_trap(
            "/ip/address/print",
            TrapResponse {
                tag: Tag::new(),
                category: None,
                message: "not enough permissions (9)".to_owned(),
            },
        );
        assert!(matches!(
            permission,
            Error::PermissionDenied { command, .. } if command == "/ip/address/print"
        ));

        let unsupported = classify_trap(
            "/interface/wifi/print",
            TrapResponse {
                tag: Tag::new(),
                category: None,
                message: "no such command".to_owned(),
            },
        );
        assert!(matches!(
            unsupported,
            Error::UnsupportedCommand { command, .. } if command == "/interface/wifi/print"
        ));

        for message in ["permission denied", "operation not permitted"] {
            assert!(matches!(
                classify_trap(
                    "/test/print",
                    TrapResponse {
                        tag: Tag::new(),
                        category: None,
                        message: message.to_owned(),
                    },
                ),
                Error::PermissionDenied { .. }
            ));
        }
        assert!(matches!(
            classify_trap(
                "/test/print",
                TrapResponse {
                    tag: Tag::new(),
                    category: None,
                    message: "unknown command name".to_owned(),
                },
            ),
            Error::UnsupportedCommand { .. }
        ));
        assert!(matches!(
            classify_trap(
                "/test/print",
                TrapResponse {
                    tag: Tag::new(),
                    category: Some(mikrotik_proto2::response::TrapCategory::GeneralFailure),
                    message: "generic failure".to_owned(),
                },
            ),
            Error::Trap {
                category: Some(mikrotik_proto2::response::TrapCategory::GeneralFailure),
                ..
            }
        ));
    }

    #[tokio::test]
    async fn raw_call_writes_a_command_and_collects_rows() {
        let (client_stream, mut server_stream) = tokio::io::duplex(4096);
        let server = tokio::spawn(async move {
            send_reply_and_done(&mut server_stream, &[b"=name=router", b"=flag="]).await;
        });
        let client = test_client(client_stream);

        assert_eq!(client.config().socket_address(), "192.0.2.1:8728");
        let rows = client
            .call("/system/identity/print", &[("detail", None)])
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get("name").map(String::as_str), Some("router"));
        assert!(!rows[0].contains_key("flag"));
        server.await.unwrap();
    }

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct TestIdentity {
        name: String,
    }

    #[tokio::test]
    async fn typed_print_deserializes_reply_rows() {
        let (client_stream, mut server_stream) = tokio::io::duplex(4096);
        let server = tokio::spawn(async move {
            send_reply_and_done(&mut server_stream, &[b"=name=router"]).await;
        });
        let client = test_client(client_stream);

        let rows = client
            .print::<TestIdentity>(PrintCommand::System(System::Identity))
            .await
            .unwrap();
        assert_eq!(
            rows,
            vec![TestIdentity {
                name: "router".to_owned()
            }]
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn generic_print_runner_returns_the_reply_row_count() {
        let (client_stream, mut server_stream) = tokio::io::duplex(4096);
        let server = tokio::spawn(async move {
            send_reply_and_done(&mut server_stream, &[b"=name=router"]).await;
        });
        let client = test_client(client_stream);

        assert_eq!(
            crate::print::run(&client, PrintCommand::System(System::Identity))
                .await
                .unwrap(),
            1
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn typed_print_reports_the_failing_row_and_command() {
        let (client_stream, mut server_stream) = tokio::io::duplex(4096);
        let server = tokio::spawn(async move {
            send_reply_and_done(&mut server_stream, &[b"=unexpected=value"]).await;
        });
        let client = test_client(client_stream);

        let error = client
            .print::<TestIdentity>(PrintCommand::System(System::Identity))
            .await
            .unwrap_err();
        let Error::Decode(error) = error else {
            panic!("expected row decode error");
        };
        assert_eq!(error.command(), "/system/identity/print");
        assert_eq!(error.row_index(), 0);
        assert_eq!(error.row().get("unexpected").map(String::as_str), Some("value"));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn call_reports_empty_trap_fatal_and_transport_close_outcomes() {
        async fn run(words: &[&[u8]]) -> Result<Vec<Row>> {
            let (client_stream, mut server_stream) = tokio::io::duplex(4096);
            let response_words = words.iter().map(|word| word.to_vec()).collect::<Vec<_>>();
            let server = tokio::spawn(async move {
                let tag = read_command_tag(&mut server_stream).await;
                let tag_word = format!(".tag={tag}");
                let mut borrowed = Vec::with_capacity(response_words.len() + 1);
                borrowed.push(response_words[0].as_slice());
                if response_words[0].as_slice() != b"!fatal" {
                    borrowed.push(tag_word.as_bytes());
                }
                borrowed.extend(response_words[1..].iter().map(Vec::as_slice));
                server_stream.write_all(&sentence(&borrowed)).await.unwrap();
            });
            let result = test_client(client_stream).call("/test/print", &[]).await;
            server.await.unwrap();
            result
        }

        assert!(run(&[b"!empty"]).await.unwrap().is_empty());
        assert!(matches!(
            run(&[b"!trap", b"=message=generic failure"]).await,
            Err(Error::Trap { command, .. }) if command == "/test/print"
        ));
        assert!(matches!(
            run(&[b"!fatal", b"shutdown"]).await,
            Err(Error::Fatal { command, reason }) if command == "/test/print" && reason == "shutdown"
        ));

        let (client_stream, server_stream) = tokio::io::duplex(64);
        drop(server_stream);
        assert!(matches!(
            test_client(client_stream).call("/test/print", &[]).await,
            Err(Error::Transport { command: Some(command), .. }) if command == "/test/print"
        ));
    }

    #[tokio::test]
    async fn call_ignores_other_command_events_and_reports_a_clean_transport_close() {
        let (client_stream, mut server_stream) = tokio::io::duplex(4096);
        let server = tokio::spawn(async move {
            let tag = read_command_tag(&mut server_stream).await;
            let wrong_tag = Tag::new();
            assert_ne!(wrong_tag, tag);
            let wrong_tag_word = format!(".tag={wrong_tag}");
            let tag_word = format!(".tag={tag}");
            let mut response = sentence(&[b"!done", wrong_tag_word.as_bytes()]);
            response.extend_from_slice(&sentence(&[b"!empty", wrong_tag_word.as_bytes()]));
            response.extend_from_slice(&sentence(&[
                b"!trap",
                wrong_tag_word.as_bytes(),
                b"=message=other command failed",
            ]));
            response.extend_from_slice(&sentence(&[b"!done", tag_word.as_bytes()]));
            server_stream.write_all(&response).await.unwrap();
        });
        assert!(
            test_client(client_stream)
                .call("/test/print", &[])
                .await
                .unwrap()
                .is_empty()
        );
        server.await.unwrap();

        let (client_stream, mut server_stream) = tokio::io::duplex(4096);
        let server = tokio::spawn(async move {
            read_command_tag(&mut server_stream).await;
        });
        assert!(matches!(
            test_client(client_stream).call("/test/print", &[]).await,
            Err(Error::ConnectionClosed { command: Some(command) }) if command == "/test/print"
        ));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn expired_attempts_and_refused_connections_exercise_retry_deadlines() {
        let expired = connect_attempt(&config(), Instant::now(), Duration::from_secs(1)).await;
        assert!(matches!(expired, Err(Error::Timeout { .. })));

        for label in [None, Some("R01")] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            drop(listener);

            let mut retry = ClientBuilder::new(
                "127.0.0.1",
                Protocol::Api,
                Credentials {
                    username: "admin".to_owned(),
                    password: None,
                },
            )
            .with_port(port)
            .with_connect_retry_timeout(Duration::from_millis(8))
            .with_connect_attempt_timeout(Duration::from_millis(2))
            .with_connect_retry_max_delay(Duration::from_millis(1));
            if let Some(label) = label {
                retry = retry.with_log_label(label);
            }
            assert!(matches!(Client::connect(retry).await, Err(Error::Transport { .. })));
        }
    }

    #[tokio::test]
    async fn unsupported_protocol_returns_without_retrying() {
        let unsupported = ClientBuilder::new(
            "192.0.2.1",
            Protocol::Ssh,
            Credentials {
                username: "admin".to_owned(),
                password: None,
            },
        );
        assert!(matches!(
            Client::connect(unsupported).await,
            Err(Error::UnsupportedProtocol("ssh"))
        ));
    }
}
