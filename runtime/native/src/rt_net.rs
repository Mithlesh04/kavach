//! `std.net` in native code (spec 14.10), with the code in `runtime/core`
//! and the same waiting as the interpreter (`compiler/interp/src/net.rs`):
//! a try that can't go on yet waits through the scheduler, until the
//! socket is ready (decision 0046) or a short sleep, and tries again. Network operations are effects, so a task running ahead
//! waits for its turn first.

use crate::heap::{Data, alloc, release};
use crate::rt::type_of;
use crate::shapes::{new_text, shape, str_of};
use kavach_kir::{RtOp, Shape, Type};
use kavach_runtime_net::net::poller::{Ready, Waits};
use kavach_runtime_net::net::{self, NetFail, Shared, Try};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// `NetDeadline`: the next network op on this thread waits at most `ms`.
pub fn set_deadline(ms: i64) -> i64 {
    net::set_next_deadline(Duration::from_millis(ms.max(0) as u64));
    0
}

fn text_type() -> Type {
    crate::heap::first_of(&Shape::Str)
}

/// 0 with the `NetError` for `f` set.
fn fail(f: NetFail, address: &str) -> i64 {
    let Some(ty) = crate::shapes::find_union("NetError", "Refused") else {
        return 0;
    };
    let text = |s: &str| new_text(s, text_type()) as u64;
    let error = match f {
        NetFail::Refused => crate::rt_json::new_variant(ty, "Refused", &[text(address)]),
        NetFail::TimedOut => crate::rt_json::new_variant(ty, "TimedOut", &[text(address)]),
        NetFail::Failed(why) => {
            crate::rt_json::new_variant(ty, "Failed", &[text(address), text(&why)])
        }
    };
    crate::rt_errors::set_error(error);
    0
}

/// Tries `once` until it is done or fails, or `deadline` passes. `Err`
/// when it failed (the error is set) or this task is cancelled. Between
/// tries it waits for `ready`'s next event, when it can tell.
fn wait<T>(
    deadline: Option<Instant>,
    address: &str,
    line: i32,
    ready: &dyn Ready,
    mut once: impl FnMut() -> Try<T>,
) -> Result<T, ()> {
    let mut n = 0;
    loop {
        let seen = ready.events();
        match once() {
            Try::Done(v) => return Ok(v),
            Try::Fail(f) => {
                fail(f, address);
                return Err(());
            }
            Try::Wait => {
                crate::rt_tasks::refuse_if_held(line);
                if deadline.is_some_and(|d| Instant::now() >= d) {
                    fail(NetFail::TimedOut, address);
                    return Err(());
                }
                if ready.waits() == Waits::No {
                    crate::rt_tasks::sleep_until(Instant::now() + net::backoff(n), line);
                } else if !crate::rt_tasks::wait_outside(deadline, line, |wake| {
                    ready.on_event(seen, wake)
                }) {
                    net::poller::wait_alone(ready, seen, deadline);
                }
                // A cancel counts even when no wait happened: the socket
                // may be ready again at once, so the loop never waits.
                crate::rt_tasks::note_cancel();
                if crate::rt_tasks::cancelling() {
                    return Err(());
                }
                n += 1;
            }
        }
    }
}

// A socket object holds its connection or listener: the code that frees
// one (and TLS with it) is in this module, so a program without
// networking doesn't link it (decision 0024).
crate::opaque::opaque_kind!(SOCKET, Shared, wrap, unwrap, |_: &Shared| String::new());

fn socket(s: Shared, ty: Type) -> i64 {
    alloc(ty, Data::Socket(wrap(Arc::new(s))))
}

/// The texts of a `[String]` (borrowed; two words an item,
/// `text_items.rs`), or none for 0.
fn texts_of(list: i64) -> Vec<String> {
    if list == 0 {
        return Vec::new();
    }
    crate::heap::slots(list)
        .chunks(crate::text_items::WIDTH)
        .map(|item| crate::text_items::with_item(item, str::to_string))
        .collect()
}

fn shared(p: i64) -> Option<Arc<Shared>> {
    match &*crate::heap::data(p).borrow() {
        Data::Socket(s) => unwrap(s),
        _ => None,
    }
}

/// The `std.net` ops (operands owned). `code` is the result type.
pub fn net_op(op: RtOp, a: i64, b: i64, c: i64, code: i64, line: i32) -> i64 {
    // A WebSocket handshake's pieces (decision 0092): no waiting.
    if op == RtOp::NetWebSocket {
        let ws = &kavach_runtime_net::websocket::accept;
        return match c {
            0 => {
                let key = str_of(a);
                release(a);
                new_text(&ws(&key), type_of(code))
            }
            1 => new_text(&kavach_runtime_net::websocket::key(), type_of(code)),
            _ => {
                let mask = kavach_runtime_net::websocket::mask().to_vec();
                crate::rt_bytes::new_bytes(mask, type_of(code))
            }
        };
    }
    let limit = net::take_next_deadline();
    crate::rt_tasks::turn();
    let deadline = Some(Instant::now() + limit.unwrap_or(net::DEADLINE));
    // `accept` and `receive` wait for as long as it takes, unless told.
    let open = limit.map(|d| Instant::now() + d);
    if matches!(
        op,
        RtOp::NetListen
            | RtOp::NetListenTls
            | RtOp::NetConnect
            | RtOp::NetConnectTls
            | RtOp::NetConnectQuic
            | RtOp::NetBind
    ) {
        let address = str_of(a);
        release(a);
        // `listen_tls` and `connect_tls` bring their other texts in a list
        // (`compiler/kir/src/lower_tls.rs`).
        let texts = texts_of(c);
        release(c);
        let text = |i: usize| texts.get(i).cloned().unwrap_or_default();
        let made = if op == RtOp::NetListenTls {
            let protocols = texts_of(b);
            release(b);
            net::listen_tls(&address, &text(0), &text(1), &protocols, &text(2))
                .map_err(|f| fail(f, &address))
                .ok()
        } else if op == RtOp::NetListen {
            net::listen(&address).map_err(|f| fail(f, &address)).ok()
        } else if op == RtOp::NetBind {
            net::bind(&address).map_err(|f| fail(f, &address)).ok()
        } else {
            let connecting = if matches!(op, RtOp::NetConnectTls | RtOp::NetConnectQuic) {
                let protocols = texts_of(b);
                release(b);
                if op == RtOp::NetConnectQuic {
                    net::connect_quic(&address, &protocols)
                } else {
                    let extra = net::ClientTls {
                        roots: text(0),
                        cert: text(1),
                        key: text(2),
                    };
                    net::connect_tls(&address, &protocols, &extra)
                }
            } else {
                net::connect(&address)
            };
            wait(deadline, &address, line, &connecting, || connecting.poll()).ok()
        };
        return made.map_or(0, |s| socket(s, type_of(code)));
    }
    let Some(s) = shared(a) else {
        release(a);
        return 0;
    };
    let ty = crate::heap::obj(a).ty();
    release(a);
    let address = s.address.clone();
    match op {
        RtOp::NetAccept => {
            wait(open, &address, line, &*s, || s.accept()).map_or(0, |c| socket(c, ty))
        }
        RtOp::NetAddress => new_text(&address, type_of(code)),
        // TLS begun on this plain connection (spec 14.10): a new
        // connection, and this one closed.
        RtOp::NetStartTls | RtOp::NetAcceptTls => {
            let protocols = texts_of(b);
            release(b);
            let texts = texts_of(c);
            release(c);
            let text = |i: usize| texts.get(i).cloned().unwrap_or_default();
            let started = if op == RtOp::NetStartTls {
                let extra = net::ClientTls {
                    roots: text(1),
                    cert: text(2),
                    key: text(3),
                };
                net::start_tls(&s, &text(0), &protocols, &extra)
            } else {
                net::accept_tls(&s, &text(0), &text(1), &protocols, &text(2))
            };
            match started {
                Ok(st) => {
                    wait(deadline, &address, line, &st, || st.poll()).map_or(0, |c| socket(c, ty))
                }
                Err(f) => {
                    fail(f, &address);
                    0
                }
            }
        }
        RtOp::NetPeerCertificate => {
            let ty = match shape(type_of(code)) {
                Shape::Optional(t) => *t,
                _ => type_of(code),
            };
            s.peer_certificate()
                .map_or(0, |der| crate::rt_bytes::new_bytes(der, ty))
        }
        RtOp::NetClientCert => {
            let ty = match shape(type_of(code)) {
                Shape::Optional(t) => *t,
                _ => type_of(code),
            };
            s.client_cert().map_or(0, |n| client_cert(&n, ty))
        }
        RtOp::NetProtocol => {
            let text = match shape(type_of(code)) {
                Shape::Optional(t) => *t,
                _ => text_type(),
            };
            s.protocol().map_or(0, |p| new_text(&p, text))
        }
        RtOp::NetRead => match wait(deadline, &address, line, &*s, || s.read()) {
            Ok(Some(data)) => {
                let item = match shape(type_of(code)) {
                    Shape::Optional(t) => *t,
                    _ => Type::Unit,
                };
                crate::rt_bytes::new_bytes(data, item)
            }
            _ => 0,
        },
        RtOp::NetWrite => {
            let data = crate::rt_bytes::with_bytes(b, <[u8]>::to_vec);
            release(b);
            let mut sent = 0;
            let _ = wait(deadline, &address, line, &*s, || s.write(&data, &mut sent));
            0
        }
        RtOp::NetSend => {
            let data = crate::rt_bytes::with_bytes(b, <[u8]>::to_vec);
            let to = str_of(c);
            release(b);
            release(c);
            let _ = wait(deadline, &to, line, &*s, || s.send_to(&data, &to));
            0
        }
        // QUIC streams.
        RtOp::NetOpenStream => {
            wait(deadline, &address, line, &*s, || s.open_stream(b != 0)).map_or(0, |n| n as i64)
        }
        RtOp::NetReadStream => {
            match wait(deadline, &address, line, &*s, || s.read_stream(b as u64)) {
                Ok(Some(data)) => {
                    let item = match shape(type_of(code)) {
                        Shape::Optional(t) => *t,
                        _ => Type::Unit,
                    };
                    crate::rt_bytes::new_bytes(data, item)
                }
                _ => 0,
            }
        }
        RtOp::NetWriteStream => {
            let (stream, done) = ((b >> 1) as u64, b & 1 == 1);
            let data = crate::rt_bytes::with_bytes(c, <[u8]>::to_vec);
            release(c);
            let mut sent = 0;
            let _ = wait(deadline, &address, line, &*s, || {
                s.write_stream(stream, &data, &mut sent, done)
            });
            0
        }
        RtOp::NetReceive => match wait(open, &address, line, &*s, || s.recv_from()) {
            Ok((data, from)) => datagram(data, &from, type_of(code)),
            Err(()) => 0,
        },
        _ => {
            s.close();
            0
        }
    }
}

/// A `ClientCert` record of type `ty` (decision 0099).
fn client_cert(n: &net::cert::CertNames, ty: Type) -> i64 {
    let Shape::Record { fields, .. } = shape(ty) else {
        return 0;
    };
    let (text_ty, list_ty) = (fields[0].ty, fields[1].ty);
    let item = match shape(list_ty) {
        Shape::List(t) => *t,
        _ => text_ty,
    };
    let list = |names: &[String]| {
        let slots = names.iter().map(|s| new_text(s, item) as u64).collect();
        crate::heap::alloc_slots(list_ty, 0, slots) as u64
    };
    let name = new_text(&n.common_name, text_ty) as u64;
    crate::heap::alloc_slots(ty, 0, vec![name, list(&n.dns), list(&n.uris)])
}

/// A `Datagram(data:, from:)` record of type `ty`.
fn datagram(data: Vec<u8>, from: &str, ty: Type) -> i64 {
    let (bytes_ty, text_ty) = match shape(ty) {
        Shape::Record { fields, .. } if fields.len() == 2 => (fields[0].ty, fields[1].ty),
        _ => (Type::Unit, text_type()),
    };
    let data = crate::rt_bytes::new_bytes(data, bytes_ty) as u64;
    let from = new_text(from, text_ty) as u64;
    crate::heap::alloc_slots(ty, 0, vec![data, from])
}

// The group's runtime entry (`RtOp::entry`, decision 0024).
crate::rt_data::entry!(kv_rt_net, |op, a, b, c, code, line| match op {
    RtOp::NetDeadline => set_deadline(a),
    _ => net_op(op, a, b, c, code, line),
});
