//! A file sent by the system (spec 14.11): on Linux, `std::io::copy`
//! from a file to a TCP socket is the kernel's `sendfile`, so the bytes go
//! from the page cache to the socket with no copy through the program.
//! Only a plain TCP connection on Linux: TLS must see the bytes, and other
//! systems send the file as it is read.

use super::{NetFail, Shared, Socket, Try};

impl Shared {
    /// Whether `send_file` sends on this connection: plain TCP, on Linux.
    pub fn can_send_file(&self) -> bool {
        cfg!(target_os = "linux") && self.with(|s| matches!(s, Socket::Stream(_)))
    }

    /// One try at sending `file`'s bytes from `*sent` to `len`, counting
    /// them in `sent`: `Wait` when the socket is full.
    #[cfg(target_os = "linux")]
    pub fn send_file(&self, file: &std::fs::File, len: u64, sent: &mut u64) -> Try<()> {
        use std::io::{ErrorKind, Read, Seek, SeekFrom};
        self.with(|s| {
            if !matches!(s, Socket::Stream(_)) {
                return Try::Fail(NetFail::Failed(
                    "this connection sends files as it reads them".into(),
                ));
            }
            let Socket::Stream(stream) = std::mem::replace(s, Socket::Closed) else {
                unreachable!("just checked");
            };
            // The same socket as a standard one, for the kernel's copy; it
            // stays registered with the poller, which knows it by number.
            let mut out: std::net::TcpStream = stream.into();
            let mut file = file;
            let result = loop {
                if *sent >= len {
                    break Try::Done(());
                }
                if let Err(e) = file.seek(SeekFrom::Start(*sent)) {
                    break Try::Fail(NetFail::Failed(e.to_string()));
                }
                let copied = std::io::copy(&mut file.take(len - *sent), &mut out);
                // The kernel moves the file's position as it sends.
                *sent = file.stream_position().unwrap_or(*sent);
                match copied {
                    Ok(0) => {
                        break Try::Fail(NetFail::Failed("the file is shorter than it was".into()));
                    }
                    Ok(_) => {}
                    Err(e) if e.kind() == ErrorKind::WouldBlock => break Try::Wait,
                    Err(e) => break Try::Fail(NetFail::Failed(e.to_string())),
                }
            };
            *s = Socket::Stream(mio::net::TcpStream::from_std(out));
            result
        })
    }

    /// Elsewhere, files are sent as they are read (`can_send_file`).
    #[cfg(not(target_os = "linux"))]
    pub fn send_file(&self, _file: &std::fs::File, _len: u64, _sent: &mut u64) -> Try<()> {
        Try::Fail(NetFail::Failed(
            "this system sends files as it reads them".into(),
        ))
    }
}
