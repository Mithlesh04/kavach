//! TLS connections (decision 0021): rustls with ring. A client checks the
//! server against Mozilla's root certificates, built in, and any roots
//! the program adds; a server proves who it is with its certificate, and
//! may ask clients for theirs (mutual TLS). The handshake runs on the
//! helper thread that connects, or while accepting; afterwards the
//! socket doesn't block, and the readiness queue wakes a task waiting
//! for it, as for plain TCP (decision 0046).

use super::{DEADLINE, NetFail, Try, fail_of};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls::{ConnectionCommon, SideData, StreamOwned};
use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;
use std::ops::DerefMut;
use std::sync::{Arc, OnceLock};

/// An encrypted connection, from either side, over a socket that
/// doesn't block.
pub(super) enum Stream {
    Client(StreamOwned<rustls::ClientConnection, mio::net::TcpStream>),
    Server(StreamOwned<rustls::ServerConnection, mio::net::TcpStream>),
}

/// The socket under `tls`, to register with the readiness queue.
pub(super) fn socket(tls: &mut Stream) -> &mut mio::net::TcpStream {
    match tls {
        Stream::Client(t) => &mut t.sock,
        Stream::Server(t) => &mut t.sock,
    }
}

/// What a client adds to the defaults (PEM text; empty when not given):
/// roots it trusts besides the built-in ones, and its own certificate
/// chain and key for servers that ask (mutual TLS).
#[derive(Clone, Default)]
pub struct ClientTls {
    pub roots: String,
    pub cert: String,
    pub key: String,
}

/// A TLS listener's settings: its certificate, and whom it asks.
pub(super) type ServerTls = Arc<rustls::ServerConfig>;

fn provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

fn bad(what: &str, e: impl std::fmt::Display) -> NetFail {
    NetFail::Failed(format!("{what}: {e}"))
}

/// The PEM certificates in `pem`.
fn certificates(pem: &str, what: &str) -> Result<Vec<CertificateDer<'static>>, NetFail> {
    let certs = CertificateDer::pem_slice_iter(pem.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| bad(what, e))?;
    if certs.is_empty() {
        return Err(NetFail::Failed(format!("{what}: no PEM certificate")));
    }
    Ok(certs)
}

/// The built-in roots, and those in `extra`.
fn roots(extra: &str) -> Result<rustls::RootCertStore, NetFail> {
    let mut roots = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    if !extra.is_empty() {
        for cert in certificates(extra, "roots")? {
            roots.add(cert).map_err(|e| bad("roots", e))?;
        }
    }
    Ok(roots)
}

/// A client's settings: the defaults, built once, unless `extra` adds to
/// them.
fn client_config(extra: &ClientTls) -> Result<Arc<rustls::ClientConfig>, NetFail> {
    static DEFAULT: OnceLock<Arc<rustls::ClientConfig>> = OnceLock::new();
    let plain = extra.roots.is_empty() && extra.cert.is_empty();
    if plain && let Some(c) = DEFAULT.get() {
        return Ok(Arc::clone(c));
    }
    let builder = rustls::ClientConfig::builder_with_provider(provider())
        .with_safe_default_protocol_versions()
        .expect("rustls' default protocol versions")
        .with_root_certificates(roots(&extra.roots)?);
    let config = if extra.cert.is_empty() {
        builder.with_no_client_auth()
    } else {
        let key = PrivateKeyDer::from_pem_slice(extra.key.as_bytes()).map_err(|e| bad("key", e))?;
        builder
            .with_client_auth_cert(certificates(&extra.cert, "cert")?, key)
            .map_err(|e| bad("cert", e))?
    };
    let config = Arc::new(config);
    if plain {
        let _ = DEFAULT.set(Arc::clone(&config));
    }
    Ok(config)
}

/// A TLS listener's settings: certificate chain `cert` and key `key`
/// (PEM), choosing from `protocols`; with `clients`, every client must
/// show a certificate signed by one of them.
pub(super) fn server_config(
    cert: &str,
    key: &str,
    protocols: &[String],
    clients: &str,
) -> Result<ServerTls, NetFail> {
    let builder = rustls::ServerConfig::builder_with_provider(provider())
        .with_safe_default_protocol_versions()
        .expect("rustls' default protocol versions");
    let builder = if clients.is_empty() {
        builder.with_no_client_auth()
    } else {
        let mut trusted = rustls::RootCertStore::empty();
        for c in certificates(clients, "clients")? {
            trusted.add(c).map_err(|e| bad("clients", e))?;
        }
        let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
            Arc::new(trusted),
            provider(),
        )
        .build()
        .map_err(|e| bad("clients", e))?;
        builder.with_client_cert_verifier(verifier)
    };
    let key = PrivateKeyDer::from_pem_slice(key.as_bytes()).map_err(|e| bad("key", e))?;
    let mut config = builder
        .with_single_cert(certificates(cert, "cert")?, key)
        .map_err(|e| bad("cert", e))?;
    config.alpn_protocols = protocols.iter().map(|p| p.as_bytes().to_vec()).collect();
    Ok(Arc::new(config))
}

/// The TLS settings inside QUIC (decision 0022): TLS 1.3 only, the same
/// roots, offering `protocols` (ALPN).
pub(super) fn quic_config(protocols: &[String]) -> rustls::ClientConfig {
    let roots = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    let mut config = rustls::ClientConfig::builder_with_provider(provider())
        .with_protocol_versions(&[&rustls::version::TLS13])
        .expect("rustls supports TLS 1.3")
        .with_root_certificates(roots)
        .with_no_client_auth();
    config.alpn_protocols = protocols.iter().map(|p| p.as_bytes().to_vec()).collect();
    config
}

/// Runs the handshake of `tls` within the deadline; then its socket
/// doesn't block.
fn finish<C, S>(
    mut tls: StreamOwned<C, TcpStream>,
) -> Result<StreamOwned<C, mio::net::TcpStream>, NetFail>
where
    C: DerefMut<Target = ConnectionCommon<S>>,
    S: SideData,
{
    tls.sock.set_read_timeout(Some(DEADLINE)).map_err(fail_of)?;
    tls.sock
        .set_write_timeout(Some(DEADLINE))
        .map_err(fail_of)?;
    while tls.conn.is_handshaking() {
        tls.conn
            .complete_io(&mut tls.sock)
            .map_err(|e| NetFail::Failed(e.to_string()))?;
    }
    let StreamOwned { conn, sock } = tls;
    sock.set_nonblocking(true).map_err(fail_of)?;
    Ok(StreamOwned::new(conn, mio::net::TcpStream::from_std(sock)))
}

/// A TLS connection over `sock` to `host` (the name its certificate must
/// have), offering `protocols` (ALPN), its handshake done.
pub(super) fn handshake(
    sock: TcpStream,
    host: &str,
    protocols: &[String],
    extra: &ClientTls,
) -> Result<Stream, NetFail> {
    let name = ServerName::try_from(host.to_string())
        .map_err(|_| NetFail::Failed(format!("`{host}` is not a host name")))?;
    let mut config = client_config(extra)?;
    if !protocols.is_empty() {
        let mut offered = (*config).clone();
        offered.alpn_protocols = protocols.iter().map(|p| p.as_bytes().to_vec()).collect();
        config = Arc::new(offered);
    }
    let conn =
        rustls::ClientConnection::new(config, name).map_err(|e| NetFail::Failed(e.to_string()))?;
    Ok(Stream::Client(finish(StreamOwned::new(conn, sock))?))
}

/// The server side of a connection a TLS listener accepted, in its
/// handshake, which goes on without blocking (`step`): a slow client
/// holds up no one.
pub(super) struct Handshake {
    tls: StreamOwned<rustls::ServerConnection, mio::net::TcpStream>,
    pub(super) started: std::time::Instant,
    pub(super) peer: String,
}

/// Starts the server side of a handshake over `sock`.
pub(super) fn start(
    sock: mio::net::TcpStream,
    config: &ServerTls,
    peer: String,
) -> Result<Handshake, NetFail> {
    let conn = rustls::ServerConnection::new(Arc::clone(config))
        .map_err(|e| NetFail::Failed(e.to_string()))?;
    Ok(Handshake {
        tls: StreamOwned::new(conn, sock),
        started: std::time::Instant::now(),
        peer,
    })
}

impl Handshake {
    /// The socket, to register with the readiness queue.
    pub(super) fn socket(&mut self) -> &mut mio::net::TcpStream {
        &mut self.tls.sock
    }

    /// Takes the handshake as far as it goes without blocking: `Done`
    /// once it is finished.
    pub(super) fn step(&mut self) -> Try<()> {
        while self.tls.conn.is_handshaking() {
            match self.tls.conn.complete_io(&mut self.tls.sock) {
                Ok(_) => {}
                Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted) => {
                    return Try::Wait;
                }
                Err(e) => return Try::Fail(NetFail::Failed(e.to_string())),
            }
        }
        Try::Done(())
    }

    /// The connection, its handshake finished.
    pub(super) fn finished(self) -> Stream {
        Stream::Server(self.tls)
    }
}

/// The certificate a client showed this server (mutual TLS), as DER.
pub(super) fn client_cert(tls: &Stream) -> Option<Vec<u8>> {
    match tls {
        Stream::Server(t) => Some(t.conn.peer_certificates()?.first()?.as_ref().to_vec()),
        Stream::Client(_) => None,
    }
}

/// The other side's certificate, as DER: the server's on a client, the
/// client's (mutual TLS) on a server.
pub(super) fn peer_cert(tls: &Stream) -> Option<Vec<u8>> {
    let certs = match tls {
        Stream::Server(t) => t.conn.peer_certificates(),
        Stream::Client(t) => t.conn.peer_certificates(),
    };
    Some(certs?.first()?.as_ref().to_vec())
}

/// The protocol the two sides agreed on.
pub(super) fn protocol(tls: &Stream) -> Option<String> {
    let p = match tls {
        Stream::Client(t) => t.conn.alpn_protocol(),
        Stream::Server(t) => t.conn.alpn_protocol(),
    }?;
    Some(String::from_utf8_lossy(p).into_owned())
}

/// One try at reading: `Done(0)` once the other side has closed.
pub(super) fn read(tls: &mut Stream, buf: &mut [u8]) -> Try<usize> {
    let got = match tls {
        Stream::Client(t) => t.read(buf),
        Stream::Server(t) => t.read(buf),
    };
    match got {
        Ok(n) => Try::Done(n),
        Err(e)
            if matches!(
                e.kind(),
                ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
            ) =>
        {
            Try::Wait
        }
        // Many servers close without saying so first.
        Err(e) if e.kind() == ErrorKind::UnexpectedEof => Try::Done(0),
        Err(e) => Try::Fail(fail_of(e)),
    }
}

/// One try at sending the rest of `data`, from `sent`: done when all of
/// it is encrypted and on its way.
pub(super) fn write(tls: &mut Stream, data: &[u8], sent: &mut usize) -> Try<()> {
    loop {
        let step = match (tls as &mut dyn Write, *sent < data.len()) {
            (w, true) => match w.write(&data[*sent..]) {
                // rustls' buffer is full: send some of it first.
                Ok(0) => w.flush(),
                n => n.map(|n| *sent += n),
            },
            (w, false) => return flushed(w.flush()),
        };
        if let Err(e) = step {
            return flushed(Err(e));
        }
    }
}

/// What a flush (or a write) that ended so means for the try.
fn flushed(r: std::io::Result<()>) -> Try<()> {
    match r {
        Ok(()) => Try::Done(()),
        Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted) => Try::Wait,
        Err(e) => Try::Fail(fail_of(e)),
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Stream::Client(t) => t.write(buf),
            Stream::Server(t) => t.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Stream::Client(t) => t.flush(),
            Stream::Server(t) => t.flush(),
        }
    }
}

pub(super) fn close(tls: &mut Stream) {
    match tls {
        Stream::Client(t) => {
            t.conn.send_close_notify();
            let _ = t.conn.complete_io(&mut t.sock);
        }
        Stream::Server(t) => {
            t.conn.send_close_notify();
            let _ = t.conn.complete_io(&mut t.sock);
        }
    }
}
