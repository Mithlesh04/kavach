//! TCP (spec 14.10), the same for the interpreter and native code. Each
//! operation is tried without blocking. When it can't go on yet, the
//! caller waits through the scheduler, so other tasks run (spec 9.7) and
//! a cancelled task stops (9.6), then tries again: a plain TCP socket
//! wakes it when the system says it is ready (`net/poller.rs`, decision
//! 0046); the others try again after a short sleep (`backoff`).
//! Connecting, which may also look the name up and do a TLS handshake
//! (`net/tls.rs`, decision 0021), runs on a helper thread that the
//! caller polls the same way. QUIC connections (`net/quic.rs`, decision
//! 0022) carry streams instead of one byte stream.

pub mod cert;
mod connect;
pub mod poller;
mod quic;
mod tls;

pub use connect::{Connecting, connect, connect_quic, connect_tls};
pub use tls::ClientTls;

use std::io::{ErrorKind, Read, Write};
use std::net::{ToSocketAddrs, UdpSocket};
use std::sync::Mutex;
use std::time::Duration;

/// How long a network wait may take before it fails with `TimedOut`
/// (all but `accept`).
pub const DEADLINE: Duration = Duration::from_secs(30);

/// The most `read` gives at a time.
const READ_SIZE: usize = 64 * 1024;

thread_local! {
    /// Where reads land, made once per thread: a read that finds nothing
    /// yet costs no memory, and one that finds 4 bytes keeps 4, not 64 KB.
    static READ_BUFFER: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Runs `read` on this thread's read buffer, and gives the first `n`
/// bytes it read, as a `Vec` of their own.
fn read_into<T>(read: impl FnOnce(&mut [u8]) -> Try<(usize, T)>) -> Try<(Vec<u8>, T)> {
    READ_BUFFER.with(|b| {
        let mut buf = b.borrow_mut();
        if buf.len() < READ_SIZE {
            buf.resize(READ_SIZE, 0);
        }
        read(&mut buf).map(|(n, extra)| (buf[..n].to_vec(), extra))
    })
}

/// How long to wait before try number `n` (from 0): 1 ms, doubling up to
/// 8 ms.
pub fn backoff(n: u32) -> Duration {
    Duration::from_millis(1 << n.min(3))
}

enum Socket {
    Listener(mio::net::TcpListener),
    /// A listener whose connections speak TLS, with its settings and the
    /// handshakes in progress.
    TlsListener(mio::net::TcpListener, tls::ServerTls, Vec<tls::Handshake>),
    Stream(mio::net::TcpStream),
    Tls(Box<tls::Stream>),
    Quic(Box<quic::Conn>),
    Udp(mio::net::UdpSocket),
    Closed,
}

/// A listener or a connection, shared by every copy of its value.
pub struct Shared {
    socket: Mutex<Socket>,
    /// Where it listens, or the other side's address (for errors).
    pub address: String,
    /// Its number with the poller, when the system says when it is ready;
    /// 0 when not. It changes when the socket moves to the queue of the
    /// worker whose task waits on it (`here`).
    token: std::sync::atomic::AtomicUsize,
}

impl Drop for Shared {
    fn drop(&mut self) {
        if let Some(n) = self.token() {
            poller::forget(n);
        }
    }
}

impl Shared {
    /// `socket`, registered with the poller when it is plain TCP.
    fn new(mut socket: Socket, address: String) -> Shared {
        let token = match &mut socket {
            Socket::Listener(l) | Socket::TlsListener(l, ..) => poller::register(l),
            Socket::Stream(s) => poller::register(s),
            Socket::Tls(t) => poller::register(tls::socket(t)),
            Socket::Udp(u) => poller::register(u),
            _ => None,
        };
        Shared {
            socket: Mutex::new(socket),
            address,
            token: std::sync::atomic::AtomicUsize::new(token.unwrap_or(0)),
        }
    }

    /// Its number with the poller, if it has one.
    pub fn token(&self) -> Option<usize> {
        Some(self.token.load(std::sync::atomic::Ordering::Acquire)).filter(|n| *n != 0)
    }

    /// How many readiness events it has had (`poller::events`).
    pub fn events(&self) -> u64 {
        self.token().map_or(0, poller::events)
    }

    /// Its number on the queue of this thread's worker. With tasks on all
    /// cores (decision 0049), a socket made by one task and used by
    /// another (a connection accepted by `serve`, read by its handler)
    /// moves to the queue of the worker that waits on it, so its events
    /// wake that worker directly. A TLS listener stays: its handshakes in
    /// progress count under its number.
    fn here(&self, n: usize) -> usize {
        if poller::is_here(n) {
            return n;
        }
        let moved = self.with(|s| match s {
            Socket::Listener(l) => poller::move_here(l, n),
            Socket::Stream(s) => poller::move_here(s, n),
            Socket::Tls(t) => poller::move_here(tls::socket(t), n),
            Socket::Udp(u) => poller::move_here(u, n),
            _ => None,
        });
        match moved {
            Some(m) => {
                self.token.store(m, std::sync::atomic::Ordering::Release);
                m
            }
            None => n,
        }
    }
}

impl poller::Ready for Shared {
    fn events(&self) -> u64 {
        Shared::events(self)
    }

    fn on_event(&self, seen: u64, wake: poller::Waker) -> bool {
        // A socket that moved has a new count: `seen` is the old one's, so
        // the caller tries again and notes the new count first.
        self.token()
            .is_some_and(|n| poller::wait(self.here(n), seen, wake))
    }

    fn waits(&self) -> poller::Waits {
        match self.token() {
            Some(_) => poller::Waits::OnQueue,
            None => poller::Waits::No,
        }
    }
}

impl std::fmt::Debug for Shared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Socket({})", self.address)
    }
}

/// Why a network operation failed: `NetError` (spec 14.10).
pub enum NetFail {
    Refused,
    TimedOut,
    Failed(String),
}

/// What one try gave: done, not yet (try again later), or failed.
pub enum Try<T> {
    Done(T),
    Wait,
    Fail(NetFail),
}

impl<T> Try<T> {
    fn map<U>(self, f: impl FnOnce(T) -> U) -> Try<U> {
        match self {
            Try::Done(v) => Try::Done(f(v)),
            Try::Wait => Try::Wait,
            Try::Fail(e) => Try::Fail(e),
        }
    }
}

fn fail_of(e: std::io::Error) -> NetFail {
    match e.kind() {
        ErrorKind::ConnectionRefused => NetFail::Refused,
        ErrorKind::TimedOut => NetFail::TimedOut,
        _ => NetFail::Failed(e.to_string()),
    }
}

fn attempt<T>(r: std::io::Result<T>) -> Try<T> {
    match r {
        Ok(v) => Try::Done(v),
        Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::Interrupted => {
            Try::Wait
        }
        Err(e) => Try::Fail(fail_of(e)),
    }
}

/// One try at a TLS listener's `accept`: new clients join the handshakes
/// in progress, under the listener's number (`token`), so any of them
/// moving on wakes the task in `accept`; the first one finished is the
/// connection.
fn tls_accept(
    l: &mut mio::net::TcpListener,
    config: &tls::ServerTls,
    pending: &mut Vec<tls::Handshake>,
    token: Option<usize>,
) -> Try<Shared> {
    loop {
        match l.accept() {
            Ok((s, peer)) => {
                let _ = s.set_nodelay(true);
                if let Ok(mut h) = tls::start(s, config, peer.to_string())
                    && token.is_none_or(|n| poller::register_as(h.socket(), n))
                {
                    pending.push(h);
                }
            }
            Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted) => break,
            Err(e) if pending.is_empty() => return Try::Fail(fail_of(e)),
            Err(_) => break,
        }
    }
    let mut i = 0;
    while i < pending.len() {
        match pending[i].step() {
            Try::Done(()) => {
                let mut h = pending.remove(i);
                let token = token.and_then(|n| poller::move_to_own(h.socket(), n));
                let address = h.peer.clone();
                return Try::Done(Shared {
                    socket: Mutex::new(Socket::Tls(Box::new(h.finished()))),
                    address,
                    token: std::sync::atomic::AtomicUsize::new(token.unwrap_or(0)),
                });
            }
            Try::Wait if pending[i].started.elapsed() < DEADLINE => i += 1,
            // Failed, or too slow: dropped.
            _ => drop(pending.remove(i)),
        }
    }
    Try::Wait
}

fn mio_stream(s: mio::net::TcpStream, address: String) -> Shared {
    let _ = s.set_nodelay(true);
    Shared::new(Socket::Stream(s), address)
}

/// `net.listen(address)`.
pub fn listen(address: &str) -> Result<Shared, NetFail> {
    let (l, address) = bind_listener(address).map_err(fail_of)?;
    Ok(Shared::new(Socket::Listener(l), address))
}

/// A listening socket at `address`, and the address it got. Connections
/// not accepted yet wait in the longest queue the system allows (decision
/// 0079): Rust's `bind` asks for 128 places, and the system drops those
/// past it, which try again only after a second or more.
fn bind_listener(address: &str) -> std::io::Result<(mio::net::TcpListener, String)> {
    use socket2::{Domain, Protocol, Socket as Raw, Type};
    let at = address_of(address)?;
    let raw = Raw::new(Domain::for_address(at), Type::STREAM, Some(Protocol::TCP))?;
    // As Rust's `bind` does: a port can be listened on again at once.
    #[cfg(unix)]
    raw.set_reuse_address(true)?;
    raw.bind(&at.into())?;
    raw.listen(i32::MAX)?;
    raw.set_nonblocking(true)?;
    let l = std::net::TcpListener::from(raw);
    let address = l
        .local_addr()
        .map_or(address.to_string(), |a| a.to_string());
    Ok((mio::net::TcpListener::from_std(l), address))
}

/// The address `to` names (`host:port`), looked up when it is a name.
fn address_of(to: &str) -> std::io::Result<std::net::SocketAddr> {
    to.to_socket_addrs()?
        .next()
        .ok_or_else(|| std::io::Error::new(ErrorKind::NotFound, "no such host"))
}

/// `net.bind(address)`: a UDP socket.
pub fn bind(address: &str) -> Result<Shared, NetFail> {
    let u = UdpSocket::bind(address).map_err(fail_of)?;
    u.set_nonblocking(true).map_err(fail_of)?;
    let address = u
        .local_addr()
        .map_or(address.to_string(), |a| a.to_string());
    Ok(Shared::new(
        Socket::Udp(mio::net::UdpSocket::from_std(u)),
        address,
    ))
}

impl Shared {
    /// One try at `sock.send(data, to)`: the whole datagram, or nothing.
    pub fn send_to(&self, data: &[u8], to: &str) -> Try<()> {
        self.with(|s| match s {
            Socket::Udp(u) => match attempt(address_of(to).and_then(|a| u.send_to(data, a))) {
                Try::Done(_) => Try::Done(()),
                Try::Wait => Try::Wait,
                Try::Fail(f) => Try::Fail(f),
            },
            _ => Try::Fail(NetFail::Failed("not a UDP socket".into())),
        })
    }

    /// One try at `sock.receive()`: a datagram and where it came from.
    pub fn recv_from(&self) -> Try<(Vec<u8>, String)> {
        self.with(|s| match s {
            Socket::Udp(u) => read_into(|buf| {
                match u.recv_from(buf) {
                    // Windows reports a datagram sent earlier to a closed
                    // port here; for UDP that means nothing.
                    Err(e) if e.kind() == ErrorKind::ConnectionReset => Try::Wait,
                    got => attempt(got).map(|(n, from)| (n, from.to_string())),
                }
            }),
            _ => Try::Fail(NetFail::Failed("not a UDP socket".into())),
        })
    }

    fn with<R>(&self, f: impl FnOnce(&mut Socket) -> R) -> R {
        f(&mut self.socket.lock().unwrap_or_else(|e| e.into_inner()))
    }

    /// One try at `listener.accept()`. A TLS listener runs the handshake
    /// too, without blocking; a client that fails it, or doesn't finish
    /// it within the deadline, is dropped, and the wait goes on.
    pub fn accept(&self) -> Try<Shared> {
        let token = self.token();
        self.with(|s| match s {
            Socket::Listener(l) => {
                attempt(l.accept()).map(|(s, peer)| mio_stream(s, peer.to_string()))
            }
            Socket::TlsListener(l, config, pending) => tls_accept(l, config, pending, token),
            _ => Try::Fail(NetFail::Failed("not listening".into())),
        })
    }

    /// One try at `conn.read()`: `None` once the other side has closed.
    pub fn read(&self) -> Try<Option<Vec<u8>>> {
        self.with(|s| match s {
            Socket::Stream(_) | Socket::Tls(_) => {
                let tried = read_into(|buf| {
                    let n = match s {
                        Socket::Tls(t) => tls::read(t, buf),
                        Socket::Stream(st) => attempt(st.read(buf)),
                        _ => Try::Wait,
                    };
                    n.map(|n| (n, ()))
                });
                tried.map(|(bytes, ())| (!bytes.is_empty()).then_some(bytes))
            }
            _ => Try::Fail(NetFail::Failed("the connection is closed".into())),
        })
    }

    /// One try at sending the rest of `data`, from `sent`; done when all
    /// of it is sent.
    pub fn write(&self, data: &[u8], sent: &mut usize) -> Try<()> {
        self.with(|s| match s {
            Socket::Stream(st) => loop {
                if *sent >= data.len() {
                    return Try::Done(());
                }
                match attempt(st.write(&data[*sent..])) {
                    Try::Done(n) => *sent += n,
                    Try::Wait => return Try::Wait,
                    Try::Fail(f) => return Try::Fail(f),
                }
            },
            Socket::Tls(t) => tls::write(t, data, sent),
            _ => Try::Fail(NetFail::Failed("the connection is closed".into())),
        })
    }

    /// `conn.client_cert()`: the names of the certificate a client showed
    /// this server (mutual TLS), if it showed one.
    pub fn client_cert(&self) -> Option<cert::CertNames> {
        self.with(|s| match s {
            Socket::Tls(t) => tls::client_cert(t).and_then(|der| cert::names(&der)),
            _ => None,
        })
    }

    /// `conn.protocol()`: what TLS agreed to speak (ALPN), if anything.
    pub fn protocol(&self) -> Option<String> {
        self.with(|s| match s {
            Socket::Tls(t) => tls::protocol(t),
            Socket::Quic(q) => q.protocol(),
            _ => None,
        })
    }

    /// `conn.close()`.
    pub fn close(&self) {
        self.with(|s| {
            match s {
                Socket::Tls(t) => tls::close(t),
                Socket::Quic(q) => q.close(),
                _ => {}
            }
            *s = Socket::Closed;
        });
    }

    fn quic<T>(&self, f: impl FnOnce(&mut quic::Conn) -> Try<T>) -> Try<T> {
        self.with(|s| match s {
            Socket::Quic(q) => f(q),
            _ => Try::Fail(NetFail::Failed("not a QUIC connection".into())),
        })
    }

    /// One try at `conn.open_stream(two_way)` (QUIC): the new stream's
    /// number.
    pub fn open_stream(&self, two_way: bool) -> Try<u64> {
        self.quic(|q| q.open(two_way))
    }

    /// One try at `conn.write_stream(stream, data, done)` (QUIC), from
    /// `sent`; done when all of it is sent.
    pub fn write_stream(&self, stream: u64, data: &[u8], sent: &mut usize, done: bool) -> Try<()> {
        self.quic(|q| q.write(stream, data, sent, done))
    }

    /// One try at `conn.read_stream(stream)` (QUIC): `None` once the other
    /// side has ended the stream.
    pub fn read_stream(&self, stream: u64) -> Try<Option<Vec<u8>>> {
        self.quic(|q| q.read(stream))
    }
}

/// `net.listen_tls(address, cert, key, protocols, clients)`.
pub fn listen_tls(
    address: &str,
    cert: &str,
    key: &str,
    protocols: &[String],
    clients: &str,
) -> Result<Shared, NetFail> {
    let config = tls::server_config(cert, key, protocols, clients)?;
    let (l, address) = bind_listener(address).map_err(fail_of)?;
    Ok(Shared::new(
        Socket::TlsListener(l, config, Vec::new()),
        address,
    ))
}

thread_local! {
    /// The deadline of this thread's next network op, from `.timeout(d)`
    /// (native code). Kept here, not in the native runtime, so its code
    /// stays with the network's (decision 0024).
    static NEXT: std::cell::Cell<Option<Duration>> = const { std::cell::Cell::new(None) };
}

/// The next network op on this thread waits at most `d`.
pub fn set_next_deadline(d: Duration) {
    NEXT.with(|n| n.set(Some(d)));
}

/// The deadline set for this op, if any; it applies once.
pub fn take_next_deadline() -> Option<Duration> {
    NEXT.with(std::cell::Cell::take)
}

#[cfg(test)]
mod tests;
