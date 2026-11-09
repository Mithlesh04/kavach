//! QUIC client connections (decision 0022): `quinn-proto`'s state machine,
//! with no I/O of its own, driven by a UDP socket and the caller's
//! try-and-wait loop, as TCP is. Every try first "pumps": it takes in the
//! datagrams that arrived, runs the timers that are due and sends what the
//! connection has to send. TLS 1.3 runs inside QUIC, checked against the
//! roots `tls.rs` uses.

use super::{DEADLINE, NetFail, Try, fail_of};
use bytes::{Bytes, BytesMut};
use quinn_proto::crypto::rustls::QuicClientConfig;
use quinn_proto::{
    ClientConfig, Connection, ConnectionHandle, DatagramEvent, Dir, Endpoint, EndpointConfig,
    Event, ReadError, StreamId, VarInt, WriteError,
};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::Arc;
use std::time::Instant;

/// The largest datagram taken in.
const DATAGRAM: usize = 64 * 1024;

pub(crate) struct Conn {
    socket: UdpSocket,
    endpoint: Endpoint,
    conn: Connection,
    handle: ConnectionHandle,
    /// The streams opened so far, by the number the program knows them by.
    streams: HashMap<u64, StreamId>,
    /// Streams the other side has ended: `quinn-proto` forgets them once
    /// read to the end, but the program still reads `none` there.
    ended: std::collections::HashSet<u64>,
    /// Why the connection ended, once it has.
    lost: Option<String>,
}

/// Connects to `address` (`host:port`), checking the certificate against
/// `host` and offering `protocols` (ALPN); waits for the handshake.
pub(super) fn connect(address: &str, protocols: &[String]) -> Result<Conn, NetFail> {
    let remote: SocketAddr = address
        .to_socket_addrs()
        .map_err(fail_of)?
        .next()
        .ok_or_else(|| NetFail::Failed("no such host".into()))?;
    let local = if remote.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    };
    let socket = UdpSocket::bind(local).map_err(fail_of)?;
    socket.set_nonblocking(true).map_err(fail_of)?;
    let tls = super::tls::quic_config(protocols);
    let crypto = QuicClientConfig::try_from(tls).map_err(|e| NetFail::Failed(e.to_string()))?;
    let config = ClientConfig::new(Arc::new(crypto));
    let mut endpoint = Endpoint::new(Arc::new(EndpointConfig::default()), None, true, None);
    let host = address.rsplit_once(':').map_or(address, |(h, _)| h);
    let host = host.trim_start_matches('[').trim_end_matches(']');
    let (handle, conn) = endpoint
        .connect(Instant::now(), config, remote, host)
        .map_err(|e| NetFail::Failed(e.to_string()))?;
    let mut c = Conn {
        socket,
        endpoint,
        conn,
        handle,
        streams: HashMap::new(),
        ended: std::collections::HashSet::new(),
        lost: None,
    };
    let started = Instant::now();
    while c.conn.is_handshaking() {
        c.pump();
        if let Some(why) = &c.lost {
            return Err(NetFail::Failed(why.clone()));
        }
        if started.elapsed() > DEADLINE {
            return Err(NetFail::TimedOut);
        }
        std::thread::sleep(super::backoff(1));
    }
    c.pump();
    match c.lost.take() {
        Some(why) => Err(NetFail::Failed(why)),
        None => Ok(c),
    }
}

impl Conn {
    /// Takes in what arrived, runs the timers that are due, and sends what
    /// the connection has to send.
    fn pump(&mut self) {
        let now = Instant::now();
        let mut buf = vec![0u8; DATAGRAM];
        loop {
            let (n, from) = match self.socket.recv_from(&mut buf) {
                Ok(got) => got,
                // A datagram sent earlier to a closed port (Windows).
                Err(e) if e.kind() == ErrorKind::ConnectionReset => continue,
                Err(_) => break,
            };
            let mut response = Vec::new();
            let data = BytesMut::from(&buf[..n]);
            match self
                .endpoint
                .handle(now, from, None, None, data, &mut response)
            {
                Some(DatagramEvent::ConnectionEvent(_, e)) => self.conn.handle_event(e),
                Some(DatagramEvent::Response(t)) => {
                    let _ = self.socket.send_to(&response[..t.size], t.destination);
                }
                _ => {}
            }
        }
        if self.conn.poll_timeout().is_some_and(|t| t <= now) {
            self.conn.handle_timeout(now);
        }
        while let Some(e) = self.conn.poll_endpoint_events() {
            if let Some(back) = self.endpoint.handle_event(self.handle, e) {
                self.conn.handle_event(back);
            }
        }
        while let Some(e) = self.conn.poll() {
            if let Event::ConnectionLost { reason } = e {
                self.lost = Some(reason.to_string());
            }
        }
        let mut out = Vec::new();
        while let Some(t) = self.conn.poll_transmit(now, 1, &mut out) {
            let _ = self.socket.send_to(&out[..t.size], t.destination);
            out.clear();
        }
    }

    /// Fails once the connection has ended.
    fn alive(&self) -> Result<(), NetFail> {
        match &self.lost {
            Some(why) => Err(NetFail::Failed(why.clone())),
            None => Ok(()),
        }
    }

    /// One try at opening a stream: two-way, or only for sending.
    pub(super) fn open(&mut self, two_way: bool) -> Try<u64> {
        self.pump();
        if let Err(f) = self.alive() {
            return Try::Fail(f);
        }
        let dir = if two_way { Dir::Bi } else { Dir::Uni };
        match self.conn.streams().open(dir) {
            Some(id) => {
                let n = u64::from(id);
                self.streams.insert(n, id);
                Try::Done(n)
            }
            // The other side's limit on open streams: wait for room.
            None => Try::Wait,
        }
    }

    fn stream(&self, n: u64) -> Result<StreamId, NetFail> {
        self.streams
            .get(&n)
            .copied()
            .ok_or_else(|| NetFail::Failed(format!("no stream {n}")))
    }

    /// One try at sending the rest of `data` (from `sent`) on stream `n`,
    /// then ending its sending side when `done`.
    pub(super) fn write(&mut self, n: u64, data: &[u8], sent: &mut usize, done: bool) -> Try<()> {
        self.pump();
        let id = match self.alive().and_then(|_| self.stream(n)) {
            Ok(id) => id,
            Err(f) => return Try::Fail(f),
        };
        while *sent < data.len() {
            match self.conn.send_stream(id).write(&data[*sent..]) {
                Ok(k) => *sent += k,
                Err(WriteError::Blocked) => {
                    self.pump();
                    return Try::Wait;
                }
                Err(e) => return Try::Fail(NetFail::Failed(e.to_string())),
            }
        }
        if done && let Err(e) = self.conn.send_stream(id).finish() {
            return Try::Fail(NetFail::Failed(e.to_string()));
        }
        self.pump();
        Try::Done(())
    }

    /// One try at reading stream `n`: what arrived, or `None` once the
    /// other side has ended the stream.
    pub(super) fn read(&mut self, n: u64) -> Try<Option<Vec<u8>>> {
        self.pump();
        let id = match self.stream(n) {
            Ok(id) => id,
            Err(f) => return Try::Fail(f),
        };
        if self.ended.contains(&n) {
            return Try::Done(None);
        }
        let mut recv = self.conn.recv_stream(id);
        let mut chunks = match recv.read(true) {
            Ok(c) => c,
            Err(e) => return Try::Fail(NetFail::Failed(e.to_string())),
        };
        let mut got = Vec::new();
        let ended = loop {
            match chunks.next(usize::MAX) {
                Ok(Some(chunk)) => got.extend_from_slice(&chunk.bytes),
                Ok(None) => break true,
                Err(ReadError::Blocked) => break false,
                Err(ReadError::Reset(code)) => {
                    let _ = chunks.finalize();
                    return Try::Fail(NetFail::Failed(format!("the stream was reset ({code})")));
                }
            }
        };
        let _ = chunks.finalize();
        if ended {
            self.ended.insert(n);
        }
        self.pump();
        match (got.is_empty(), ended) {
            (false, _) => Try::Done(Some(got)),
            (true, true) => Try::Done(None),
            (true, false) => match self.alive() {
                Ok(()) => Try::Wait,
                Err(f) => Try::Fail(f),
            },
        }
    }

    /// The protocol the server chose from those offered (ALPN).
    pub(super) fn protocol(&self) -> Option<String> {
        let data = self.conn.crypto_session().handshake_data()?;
        let data = data
            .downcast::<quinn_proto::crypto::rustls::HandshakeData>()
            .ok()?;
        data.protocol
            .map(|p| String::from_utf8_lossy(&p).into_owned())
    }

    pub(super) fn close(&mut self) {
        self.conn
            .close(Instant::now(), VarInt::from_u32(0), Bytes::new());
        self.pump();
    }
}

#[cfg(test)]
mod tests {
    /// Needs the internet: `cargo test -p kavach-runtime-net -- --ignored`.
    #[test]
    #[ignore]
    fn a_quic_handshake_agrees_on_http3() {
        let c = super::connect("cloudflare-quic.com:443", &["h3".to_string()])
            .unwrap_or_else(|_| panic!("no QUIC connection"));
        assert_eq!(c.protocol().as_deref(), Some("h3"));
    }
}
