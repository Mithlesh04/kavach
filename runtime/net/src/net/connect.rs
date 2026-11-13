//! Connecting (spec 14.10): looking the name up, connecting and the TLS
//! or QUIC handshake run on a helper thread. When it is done, the helper
//! wakes the task waiting for it (decision 0046), and interrupts the
//! readiness queue in case the scheduler's thread waits on it.

use super::poller::{self, Ready, Waits, Waker};
use super::{ClientTls, DEADLINE, NetFail, Shared, Socket, Try, fail_of, quic, tls};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex, mpsc};

/// `net.connect(address)` or `net.connect_tls(address)` in progress, on
/// a helper thread.
pub struct Connecting {
    done: mpsc::Receiver<Result<Socket, NetFail>>,
    address: String,
    /// Whether the helper has finished, and the waker of a task waiting.
    notice: Arc<Mutex<(bool, Option<Waker>)>>,
}

pub fn connect(address: &str) -> Connecting {
    connect_with(address, None)
}

/// `net.connect_tls(address, protocols, …)`: TLS, the certificate checked
/// against the host name in `address`, offering `protocols` (ALPN), with
/// `extra` roots and a client certificate when given.
pub fn connect_tls(address: &str, protocols: &[String], extra: &ClientTls) -> Connecting {
    connect_with(address, Some((protocols.to_vec(), extra.clone())))
}

/// `net.connect_quic(address, protocols)`: QUIC with TLS 1.3, checked
/// like `connect_tls`, offering `protocols` (ALPN, decision 0022).
pub fn connect_quic(address: &str, protocols: &[String]) -> Connecting {
    let (to, protocols) = (address.to_string(), protocols.to_vec());
    on_helper(address, move || {
        quic::connect(&to, &protocols).map(|q| Socket::Quic(Box::new(q)))
    })
}

/// Connects, with TLS when `tls` holds the protocols to offer and the
/// client's settings.
fn connect_with(address: &str, tls: Option<(Vec<String>, ClientTls)>) -> Connecting {
    let to = address.to_string();
    on_helper(address, move || {
        let tcp = to.to_socket_addrs().map_err(fail_of).and_then(|mut addrs| {
            let a = addrs
                .next()
                .ok_or_else(|| NetFail::Failed("no such host".into()))?;
            TcpStream::connect_timeout(&a, DEADLINE).map_err(fail_of)
        });
        tcp.and_then(|s| {
            let Some((protocols, extra)) = &tls else {
                s.set_nonblocking(true).map_err(fail_of)?;
                let _ = s.set_nodelay(true);
                return Ok(Socket::Stream(mio::net::TcpStream::from_std(s)));
            };
            let host = to.rsplit_once(':').map_or(to.as_str(), |(h, _)| h);
            Ok(Socket::Tls(Box::new(tls::handshake(
                s, host, protocols, extra,
            )?)))
        })
    })
}

/// Runs `work` on a helper thread, which wakes the waiting task when done.
pub(super) fn on_helper(
    address: &str,
    work: impl FnOnce() -> Result<Socket, NetFail> + Send + 'static,
) -> Connecting {
    let (send, done) = mpsc::channel();
    let notice = Arc::new(Mutex::new((false, None)));
    let told = Arc::clone(&notice);
    std::thread::spawn(move || {
        let _ = send.send(work());
        let waker: Option<Waker> = {
            let mut n = told.lock().unwrap_or_else(|e| e.into_inner());
            n.0 = true;
            n.1.take()
        };
        if let Some(wake) = waker {
            wake();
            poller::interrupt();
        }
    });
    Connecting {
        done,
        address: address.to_string(),
        notice,
    }
}

impl Connecting {
    /// One look at whether connecting has finished.
    pub fn poll(&self) -> Try<Shared> {
        match self.done.try_recv() {
            Ok(Ok(socket)) => Try::Done(Shared::new(socket, self.address.clone())),
            Ok(Err(f)) => Try::Fail(f),
            Err(mpsc::TryRecvError::Empty) => Try::Wait,
            Err(mpsc::TryRecvError::Disconnected) => Try::Fail(NetFail::Failed("connect".into())),
        }
    }
}

impl Ready for Connecting {
    fn events(&self) -> u64 {
        u64::from(self.notice.lock().unwrap_or_else(|e| e.into_inner()).0)
    }

    fn on_event(&self, seen: u64, wake: Waker) -> bool {
        let mut n = self.notice.lock().unwrap_or_else(|e| e.into_inner());
        if u64::from(n.0) != seen {
            return false;
        }
        n.1 = Some(wake);
        true
    }

    fn waits(&self) -> Waits {
        Waits::OnThread
    }
}
