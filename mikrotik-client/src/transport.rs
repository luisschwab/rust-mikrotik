//! Tokio transport for the sans-IO `RouterOS` protocol state machines.

use core::fmt;
use core::result;
use std::sync::Arc;

use mikrotik_proto2::Connection;
use mikrotik_proto2::LoginProgress;
use mikrotik_proto2::handshake::Handshaking;
use rustls::ClientConfig;
use rustls::DigitallySignedStruct;
use rustls::Error as RustlsError;
use rustls::SignatureScheme;
use rustls::client::danger::HandshakeSignatureValid;
use rustls::client::danger::ServerCertVerified;
use rustls::client::danger::ServerCertVerifier;
use rustls::crypto::CryptoProvider;
use rustls::pki_types::CertificateDer;
use rustls::pki_types::ServerName;
use rustls::pki_types::UnixTime;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

use crate::builder::ClientBuilder;
use crate::builder::Protocol;
use crate::error::Error;
use crate::error::Result;

/// Trait alias for streams that can carry `RouterOS` API frames.
pub(crate) trait AsyncStream: AsyncRead + AsyncWrite + Unpin + Send {}

impl<T> AsyncStream for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

/// Authenticated transport plus sans-IO protocol state.
pub(crate) struct Session {
    /// TCP or TLS stream connected to `RouterOS`.
    pub(crate) stream: Box<dyn AsyncStream>,
    /// Sans-IO connection state machine.
    pub(crate) connection: Connection,
}

impl Session {
    /// Connect to a `RouterOS` API endpoint and complete login.
    pub(crate) async fn connect(config: &ClientBuilder) -> Result<Self> {
        let stream = connect_stream(config).await?;
        let connection = login(stream, config).await?;

        Ok(connection)
    }
}

impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session").finish_non_exhaustive()
    }
}

/// Open the configured TCP or TLS stream.
async fn connect_stream(config: &ClientBuilder) -> Result<Box<dyn AsyncStream>> {
    match config.protocol {
        Protocol::Api => {
            let tcp_stream = TcpStream::connect(config.socket_address()).await?;
            tcp_stream.set_nodelay(true)?;
            let stream: Box<dyn AsyncStream> = Box::new(tcp_stream);
            Ok(stream)
        }
        Protocol::ApiSsl => {
            let tcp_stream = TcpStream::connect(config.socket_address()).await?;
            tcp_stream.set_nodelay(true)?;
            let connector = TlsConnector::from(insecure_client_config());
            let server_name = ServerName::try_from("mikrotik").expect("\"mikrotik\" is a valid DNS name");
            let stream = connector.connect(server_name, tcp_stream).await?;
            let stream: Box<dyn AsyncStream> = Box::new(stream);
            Ok(stream)
        }
        Protocol::Ssh => Err(Error::UnsupportedProtocol("ssh")),
        Protocol::Telnet => Err(Error::UnsupportedProtocol("telnet")),
        Protocol::Ftp => Err(Error::UnsupportedProtocol("ftp")),
        Protocol::Http => Err(Error::UnsupportedProtocol("http")),
        Protocol::Https => Err(Error::UnsupportedProtocol("https")),
        Protocol::WinBox => Err(Error::UnsupportedProtocol("winbox")),
        Protocol::MacTelnet => Err(Error::UnsupportedProtocol("mac-telnet")),
    }
}

/// Drive the `RouterOS` login handshake over an open stream.
async fn login(mut stream: Box<dyn AsyncStream>, config: &ClientBuilder) -> Result<Session> {
    let mut handshaking = Handshaking::new(&config.credentials.username, config.credentials.password.as_deref())?;

    flush_login_transmits(&mut *stream, &mut handshaking).await?;

    let mut buffer = [0u8; 4096];
    let connection = loop {
        let read = stream.read(&mut buffer).await?;
        if read == 0 {
            return Err(Error::ConnectionClosed { command: None });
        }

        handshaking.receive(&buffer[..read])?;
        flush_login_transmits(&mut *stream, &mut handshaking).await?;

        match handshaking.advance()? {
            LoginProgress::Pending(next) => handshaking = next,
            LoginProgress::Complete(authenticated) => break authenticated.into_connection(),
        }
    };

    Ok(Session { stream, connection })
}

/// Write all pending login handshake transmissions.
async fn flush_login_transmits(stream: &mut dyn AsyncStream, handshaking: &mut Handshaking) -> Result<()> {
    while let Some(transmit) = handshaking.poll_transmit() {
        stream.write_all(&transmit.data).await?;
    }
    Ok(())
}

/// Certificate verifier that accepts any server certificate.
#[derive(Debug)]
struct NoVerifier(Arc<CryptoProvider>);

impl ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> result::Result<ServerCertVerified, RustlsError> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> result::Result<HandshakeSignatureValid, RustlsError> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> result::Result<HandshakeSignatureValid, RustlsError> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

/// Build a TLS client configuration matching `RouterOS` API-SSL's local-test needs.
fn insecure_client_config() -> Arc<ClientConfig> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());

    let config = ClientConfig::builder_with_provider(Arc::clone(&provider))
        .with_safe_default_protocol_versions()
        .expect("the ring provider supports rustls default protocol versions")
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoVerifier(provider)))
        .with_no_client_auth();

    Arc::new(config)
}

#[cfg(test)]
mod tests {
    use mikrotik_proto2::codec;
    use mikrotik_proto2::codec::Decode;
    use mikrotik_proto2::word::Word;
    use mikrotik_types::target::Credentials;

    use super::*;

    fn config(protocol: Protocol) -> ClientBuilder {
        ClientBuilder::new(
            "192.0.2.1",
            protocol,
            Credentials {
                username: "admin".to_owned(),
                password: None,
            },
        )
    }

    async fn read_login_tag(stream: &mut tokio::io::DuplexStream) -> mikrotik_proto2::Tag {
        let mut data = Vec::new();
        loop {
            let mut buffer = [0; 512];
            let read = stream.read(&mut buffer).await.unwrap();
            assert_ne!(read, 0);
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
                .unwrap();
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

    #[tokio::test]
    async fn unsupported_transport_protocols_fail_before_network_io() {
        for (protocol, expected) in [
            (Protocol::Ssh, "ssh"),
            (Protocol::Telnet, "telnet"),
            (Protocol::Ftp, "ftp"),
            (Protocol::Http, "http"),
            (Protocol::Https, "https"),
            (Protocol::WinBox, "winbox"),
            (Protocol::MacTelnet, "mac-telnet"),
        ] {
            assert!(matches!(
                connect_stream(&config(protocol)).await,
                Err(Error::UnsupportedProtocol(value)) if value == expected
            ));
        }
    }

    #[tokio::test]
    async fn login_flushes_the_request_and_returns_an_active_session() {
        let (client_stream, mut server_stream) = tokio::io::duplex(4096);
        let server = tokio::spawn(async move {
            let tag = read_login_tag(&mut server_stream).await;
            let tag_word = format!(".tag={tag}");
            server_stream
                .write_all(&sentence(&[b"!done", tag_word.as_bytes()]))
                .await
                .unwrap();
        });

        let session = login(Box::new(client_stream), &config(Protocol::Api)).await.unwrap();
        assert!(session.connection.is_active());
        assert_eq!(session.connection.in_flight_count(), 0);
        assert_eq!(format!("{session:?}"), "Session { .. }");
        server.await.unwrap();
    }

    #[tokio::test]
    async fn login_reports_a_peer_that_closes_before_replying() {
        let (client_stream, mut server_stream) = tokio::io::duplex(4096);
        let server = tokio::spawn(async move {
            let _ = read_login_tag(&mut server_stream).await;
        });

        let error = login(Box::new(client_stream), &config(Protocol::Api))
            .await
            .unwrap_err();
        assert!(matches!(error, Error::ConnectionClosed { command: None }));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn login_remains_pending_for_reply_rows_before_completion() {
        let (client_stream, mut server_stream) = tokio::io::duplex(4096);
        let server = tokio::spawn(async move {
            let tag = read_login_tag(&mut server_stream).await;
            let tag_word = format!(".tag={tag}");
            server_stream
                .write_all(&sentence(&[
                    b"!re",
                    tag_word.as_bytes(),
                    b"=message=still authenticating",
                ]))
                .await
                .unwrap();
            tokio::task::yield_now().await;
            server_stream
                .write_all(&sentence(&[b"!done", tag_word.as_bytes()]))
                .await
                .unwrap();
        });

        let session = login(Box::new(client_stream), &config(Protocol::Api)).await.unwrap();
        assert!(session.connection.is_active());
        server.await.unwrap();
    }

    #[tokio::test]
    async fn api_ssl_transport_rejects_non_tls_peers_after_tcp_connect() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            stream.write_all(&[0xff; 64]).await.unwrap();
        });
        let tls = ClientBuilder::new(
            "127.0.0.1",
            Protocol::ApiSsl,
            Credentials {
                username: "admin".to_owned(),
                password: None,
            },
        )
        .with_port(port);
        assert!(connect_stream(&tls).await.is_err());
        server.await.unwrap();
    }

    #[test]
    fn insecure_tls_config_supports_routeros_signature_schemes() {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let verifier = NoVerifier(Arc::clone(&provider));
        assert!(!verifier.supported_verify_schemes().is_empty());
        assert!(
            verifier
                .verify_server_cert(
                    &CertificateDer::from(&[][..]),
                    &[],
                    &ServerName::try_from("mikrotik").unwrap(),
                    &[],
                    UnixTime::since_unix_epoch(core::time::Duration::ZERO),
                )
                .is_ok()
        );

        let config = insecure_client_config();
        assert!(
            !config
                .crypto_provider()
                .signature_verification_algorithms
                .supported_schemes()
                .is_empty()
        );
    }
}
