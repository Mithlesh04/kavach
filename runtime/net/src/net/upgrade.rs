//! TLS begun on a connection that already spoke plain TCP (spec 14.10):
//! `conn.start_tls(host, …)` as a client, `conn.accept_tls(cert, key, …)`
//! as a server, the way PostgreSQL's `SSLRequest` and SMTP's `STARTTLS`
//! do it. The plain stream is taken out of its value, which is closed from
//! then on, and the handshake runs on a helper thread, as `connect_tls`'s
//! does.

use super::connect::on_helper;
use super::{ClientTls, Connecting, NetFail, Shared, Socket, fail_of, poller, tls};
use std::net::TcpStream;

/// The connection's plain TCP stream, taken out of `conn` and off the
/// poller's queue, as a blocking standard stream for the handshake.
fn take_plain(conn: &Shared) -> Result<TcpStream, NetFail> {
    let token = conn.token();
    let stream = conn.with(|s| match std::mem::replace(s, Socket::Closed) {
        Socket::Stream(mut m) => {
            if let Some(n) = token {
                poller::deregister(&mut m, n);
            }
            Ok(m)
        }
        other => {
            *s = other;
            Err(NetFail::Failed(
                "TLS starts only on a plain TCP connection".into(),
            ))
        }
    })?;
    conn.token.store(0, std::sync::atomic::Ordering::Release);
    let stream = into_std(stream);
    stream.set_nonblocking(false).map_err(fail_of)?;
    Ok(stream)
}

#[cfg(unix)]
fn into_std(m: mio::net::TcpStream) -> TcpStream {
    TcpStream::from(std::os::fd::OwnedFd::from(m))
}

#[cfg(windows)]
fn into_std(m: mio::net::TcpStream) -> TcpStream {
    TcpStream::from(std::os::windows::io::OwnedSocket::from(m))
}

/// `conn.start_tls(host, protocols, roots, cert, key)`: the client's side,
/// the server's certificate checked against `host`.
pub fn start_tls(
    conn: &Shared,
    host: &str,
    protocols: &[String],
    extra: &ClientTls,
) -> Result<Connecting, NetFail> {
    let stream = take_plain(conn)?;
    let (host, protocols, extra) = (host.to_string(), protocols.to_vec(), extra.clone());
    Ok(on_helper(&conn.address, move || {
        Ok(Socket::Tls(Box::new(tls::handshake(
            stream, &host, &protocols, &extra,
        )?)))
    }))
}

/// `conn.accept_tls(cert, key, protocols, clients)`: the server's side.
pub fn accept_tls(
    conn: &Shared,
    cert: &str,
    key: &str,
    protocols: &[String],
    clients: &str,
) -> Result<Connecting, NetFail> {
    let config = tls::server_config(cert, key, protocols, clients)?;
    let stream = take_plain(conn)?;
    Ok(on_helper(&conn.address, move || {
        Ok(Socket::Tls(Box::new(tls::server_handshake(
            stream, config,
        )?)))
    }))
}
