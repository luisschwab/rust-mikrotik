//! Tokio transport for the sans-IO `RouterOS` protocol state machines.

use core::fmt;
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
            return Err(Error::ConnectionClosed);
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
    ) -> core::result::Result<ServerCertVerified, RustlsError> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> core::result::Result<HandshakeSignatureValid, RustlsError> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> core::result::Result<HandshakeSignatureValid, RustlsError> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.0.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

/// Build a TLS client configuration matching `RouterOS` API-SSL's local-test needs.
fn insecure_client_config() -> Arc<ClientConfig> {
    let provider = CryptoProvider::get_default()
        .cloned()
        .expect("rustls AWS-LC crypto provider should be installed before connecting");

    let config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoVerifier(provider)))
        .with_no_client_auth();

    Arc::new(config)
}
