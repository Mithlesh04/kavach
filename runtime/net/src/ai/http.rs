// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A small HTTP/1.1 client for the model call: one POST on a connection
//! that closes after it. It runs on the call's helper thread, so it may
//! block; `wait` tries the non-blocking network steps until a deadline.

use crate::net::{self, NetFail, Try};
use std::time::Instant;

/// Why no reply came.
pub(crate) enum Trouble {
    /// The `timeout` passed.
    TimedOut(String),
    /// No connection, or it dropped.
    Network(String),
}

/// A reply: its status, headers (names in lowercase) and body.
pub(crate) type Whole = (i64, Vec<(String, String)>, Vec<u8>);

/// Sends `body` to `url` (`http://` or `https://`) and gives the status,
/// headers and body of the response.
pub(crate) fn post(
    url: &str,
    headers: &[(&str, &str)],
    body: &[u8],
    timeout: std::time::Duration,
) -> Result<Whole, Trouble> {
    let (secure, rest) = if let Some(r) = url.strip_prefix("https://") {
        (true, r)
    } else if let Some(r) = url.strip_prefix("http://") {
        (false, r)
    } else {
        return Err(Trouble::Network(format!(
            "`{url}` is not an http:// or https:// address"
        )));
    };
    let slash = rest.find('/').unwrap_or(rest.len());
    let host = &rest[..slash];
    let path = if slash < rest.len() {
        &rest[slash..]
    } else {
        "/"
    };
    let port = if secure { 443 } else { 80 };
    let address = if host.contains(':') {
        host.to_string()
    } else {
        format!("{host}:{port}")
    };
    let name = host.split(':').next().unwrap_or(host);
    let deadline = Instant::now() + timeout;
    let connecting = if secure {
        net::connect_tls(
            &address,
            &["http/1.1".to_string()],
            &net::ClientTls::default(),
        )
    } else {
        net::connect(&address)
    };
    let conn = wait(deadline, || connecting.poll())?;
    let mut head = format!(
        "POST {path} HTTP/1.1\r\nhost: {name}\r\ncontent-length: {}\r\nconnection: close\r\n",
        body.len()
    );
    for (k, v) in headers {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str("\r\n");
    let mut data = head.into_bytes();
    data.extend_from_slice(body);
    let mut sent = 0;
    wait(deadline, || conn.write(&data, &mut sent))?;
    let mut reply = Vec::new();
    while let Some(piece) = wait(deadline, || conn.read())? {
        reply.extend(piece);
    }
    conn.close();
    parse(&reply).map_err(|m| Trouble::Network(m.to_string()))
}

/// Tries `once` until it is done or fails, or `deadline` passes.
fn wait<T>(deadline: Instant, mut once: impl FnMut() -> Try<T>) -> Result<T, Trouble> {
    let mut n = 0;
    loop {
        match once() {
            Try::Done(v) => return Ok(v),
            Try::Fail(NetFail::Refused) => {
                return Err(Trouble::Network("the connection was refused".into()));
            }
            Try::Fail(NetFail::TimedOut) => {
                return Err(Trouble::TimedOut("the network timed out".into()));
            }
            Try::Fail(NetFail::Failed(why)) => return Err(Trouble::Network(why)),
            Try::Wait => {
                if Instant::now() >= deadline {
                    return Err(Trouble::TimedOut("the call took too long".into()));
                }
                std::thread::sleep(net::backoff(n));
                n += 1;
            }
        }
    }
}

/// The status, headers and body of a whole response.
pub(super) fn parse(reply: &[u8]) -> Result<Whole, &'static str> {
    let end = reply
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("the reply is not HTTP")?;
    let head = String::from_utf8_lossy(&reply[..end]);
    let mut lines = head.split("\r\n");
    let status = lines
        .next()
        .and_then(|l| l.split(' ').nth(1))
        .and_then(|s| s.parse().ok())
        .ok_or("the reply has no status")?;
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    let chunked = (headers.iter())
        .any(|(k, v)| k == "transfer-encoding" && v.to_ascii_lowercase().contains("chunked"));
    let body = &reply[end + 4..];
    Ok((
        status,
        headers,
        if chunked {
            dechunk(body)?
        } else {
            body.to_vec()
        },
    ))
}

/// A body sent in chunks: each a length in hex, a line end, that many
/// bytes and a line end; a length of 0 ends it.
pub(super) fn dechunk(mut data: &[u8]) -> Result<Vec<u8>, &'static str> {
    let mut out = Vec::new();
    loop {
        let line = data
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or("a chunk is cut off")?;
        let size_text = String::from_utf8_lossy(&data[..line]);
        let size_text = size_text.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_text, 16).map_err(|_| "a chunk has a bad size")?;
        data = &data[line + 2..];
        if size == 0 {
            return Ok(out);
        }
        if data.len() < size + 2 {
            return Err("a chunk is cut off");
        }
        out.extend_from_slice(&data[..size]);
        data = &data[size + 2..];
    }
}
