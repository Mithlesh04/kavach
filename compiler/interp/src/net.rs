//! `std.net` (spec 14.10): TCP with the code in `runtime/core`, the same as
//! native code (`runtime/native/src/rt_net.rs`). A wait lasts until the
//! socket is ready (decision 0046), or a short sleep for the sockets the
//! poller doesn't watch, through the scheduler: other tasks run meanwhile.

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_ast::Span;
use kavach_runtime_net::net::poller::{Ready, Waits};
use kavach_runtime_net::net::{self, NetFail, Shared, Try};
use std::sync::Arc;
use std::time::{Duration, Instant};

impl<'a> Interp<'a> {
    fn net_error(&self, f: NetFail, address: &str) -> Flow<'a> {
        let at = ("address", self.str_value(address));
        let error = match f {
            NetFail::Refused => self.builtin_variant("NetError", "Refused", vec![at]),
            NetFail::TimedOut => self.builtin_variant("NetError", "TimedOut", vec![at]),
            NetFail::Failed(reason) => {
                let why = ("reason", self.str_value(&reason));
                self.builtin_variant("NetError", "Failed", vec![at, why])
            }
        };
        Flow::Fail(error)
    }

    /// Tries `once` until it is done or fails, or `deadline` passes.
    /// Between tries it waits for `ready`'s next event, when it can tell.
    fn net_wait<T>(
        &mut self,
        deadline: Option<Instant>,
        address: &str,
        span: Span,
        ready: &dyn Ready,
        mut once: impl FnMut() -> Try<T>,
    ) -> Result<T, Flow<'a>> {
        let mut n = 0;
        loop {
            let seen = ready.events();
            match once() {
                Try::Done(v) => return Ok(v),
                Try::Fail(f) => return Err(self.net_error(f, address)),
                Try::Wait => {
                    if deadline.is_some_and(|d| Instant::now() >= d) {
                        return Err(self.net_error(NetFail::TimedOut, address));
                    }
                    match ready.waits() {
                        Waits::No => self.sleep_until(Instant::now() + net::backoff(n), span)?,
                        _ => self.wait_ready(ready, seen, deadline, span)?,
                    }
                    n += 1;
                }
            }
        }
    }

    /// Waits until `ready` has had an event after the count `seen`
    /// (`net::poller`), or until `at`; other tasks run meanwhile.
    fn wait_ready(
        &mut self,
        ready: &dyn Ready,
        seen: u64,
        at: Option<Instant>,
        span: Span,
    ) -> Result<(), Flow<'a>> {
        self.refuse_if_held(span)?;
        let Some(sched) = self.tasks.as_ref().map(|t| Arc::clone(&t.sched)) else {
            net::poller::wait_alone(ready, seen, at);
            return Ok(());
        };
        self.flush();
        let me = self.task;
        let (wait, waker) = (sched.next_wait(me), Arc::clone(&sched));
        if !ready.on_event(seen, Box::new(move || waker.wake_wait(me, wait))) {
            return Ok(());
        }
        sched
            .wait_io(me, at)
            .map_err(|stop| self.stopped(stop, span))
    }

    /// `net.listen(address)`, `net.connect(address)` and the others;
    /// `limit` is a connect's deadline from `.timeout(d)` (30 seconds
    /// without).
    pub(crate) fn net_call(
        &mut self,
        name: &str,
        args: &[Value<'a>],
        limit: Option<Duration>,
        span: Span,
    ) -> Eval<'a> {
        let address = match args.first() {
            Some(Value::Str(s)) => s.to_string(),
            _ => String::new(),
        };
        // A WebSocket handshake's pieces (decision 0092): no network.
        {
            use kavach_runtime_net::websocket;
            match name {
                "websocket_accept" => return Ok(self.str_value(&websocket::accept(&address))),
                "websocket_key" => return Ok(self.str_value(&websocket::key())),
                "websocket_mask" => return Ok(Value::Bytes(websocket::mask().as_slice().into())),
                _ => {}
            }
        }
        let address = address.as_str();
        // A generated test never reaches the network (spec 11.5).
        if self.fake.is_some() {
            return self.fake_net(name, address);
        }
        // `connect_tls`'s protocols to offer (ALPN).
        let protocols = strings(args.get(1));
        let text = |i: usize| match args.get(i) {
            Some(Value::Str(s)) => s.to_string(),
            _ => String::new(),
        };
        let socket = if name == "listen" {
            net::listen(address).map_err(|f| self.net_error(f, address))?
        } else if name == "listen_tls" {
            // `(address, cert, key, protocols, clients)`, arranged by `tls_args`.
            let protocols = strings(args.get(3));
            net::listen_tls(address, &text(1), &text(2), &protocols, &text(4))
                .map_err(|f| self.net_error(f, address))?
        } else if name == "bind" {
            net::bind(address).map_err(|f| self.net_error(f, address))?
        } else {
            let connecting = match name {
                "connect_tls" => {
                    let extra = net::ClientTls {
                        roots: text(2),
                        cert: text(3),
                        key: text(4),
                    };
                    net::connect_tls(address, &protocols, &extra)
                }
                "connect_quic" => net::connect_quic(address, &protocols),
                _ => net::connect(address),
            };
            let deadline = Some(Instant::now() + limit.unwrap_or(net::DEADLINE));
            self.net_wait(deadline, address, span, &connecting, || connecting.poll())?
        };
        Ok(Value::Socket(Arc::new(socket)))
    }

    /// The methods of a socket; `limit` is the deadline from
    /// `.timeout(d)`.
    pub(crate) fn socket_method(
        &mut self,
        s: &Arc<Shared>,
        name: &str,
        args: &[Value<'a>],
        limit: Option<Duration>,
        span: Span,
    ) -> Eval<'a> {
        let deadline = Some(Instant::now() + limit.unwrap_or(net::DEADLINE));
        // `accept` and `receive` wait for as long as it takes, unless told.
        let open = limit.map(|d| Instant::now() + d);
        let address = s.address.clone();
        Ok(match name {
            "accept" => {
                let conn = self.net_wait(open, &address, span, &**s, || s.accept())?;
                Value::Socket(Arc::new(conn))
            }
            "address" | "peer" => self.str_value(&address),
            "protocol" => match s.protocol() {
                Some(p) => self.str_value(&p),
                None => Value::None,
            },
            "read" => match self.net_wait(deadline, &address, span, &**s, || s.read())? {
                Some(data) => Value::Bytes(data.into()),
                None => Value::None,
            },
            "write" => {
                let data: std::rc::Rc<[u8]> = match args.first() {
                    Some(Value::Bytes(b)) => b.clone(),
                    _ => std::rc::Rc::from(Vec::new()),
                };
                let mut sent = 0;
                self.net_wait(deadline, &address, span, &**s, || s.write(&data, &mut sent))?;
                Value::Unit
            }
            "close" => {
                s.close();
                Value::Unit
            }
            // QUIC streams.
            "open_stream" => {
                let two_way = matches!(args.first(), Some(Value::Bool(true)));
                let n = self.net_wait(deadline, &address, span, &**s, || s.open_stream(two_way))?;
                Value::Int(i128::from(n), kavach_checker::Prim::Int)
            }
            "write_stream" | "read_stream" => {
                let stream = match args.first() {
                    Some(Value::Int(n, _)) => *n as u64,
                    _ => 0,
                };
                if name == "read_stream" {
                    let got =
                        self.net_wait(deadline, &address, span, &**s, || s.read_stream(stream))?;
                    return Ok(got.map_or(Value::None, |d| Value::Bytes(d.into())));
                }
                let data: std::rc::Rc<[u8]> = match args.get(1) {
                    Some(Value::Bytes(b)) => b.clone(),
                    _ => std::rc::Rc::from(Vec::new()),
                };
                let done = matches!(args.get(2), Some(Value::Bool(true)));
                let mut sent = 0;
                self.net_wait(deadline, &address, span, &**s, || {
                    s.write_stream(stream, &data, &mut sent, done)
                })?;
                Value::Unit
            }
            // UDP.
            "send" => {
                let data: std::rc::Rc<[u8]> = match args.first() {
                    Some(Value::Bytes(b)) => b.clone(),
                    _ => std::rc::Rc::from(Vec::new()),
                };
                let to = match args.get(1) {
                    Some(Value::Str(t)) => t.to_string(),
                    _ => String::new(),
                };
                self.net_wait(deadline, &to, span, &**s, || s.send_to(&data, &to))?;
                Value::Unit
            }
            "receive" => {
                let (data, from) = self.net_wait(open, &address, span, &**s, || s.recv_from())?;
                let Some(def) = self.c.resolution.std_type("Datagram") else {
                    return Err(self.panic("`Datagram` is missing", span));
                };
                let field = |name: &str, value| crate::value::Field {
                    name: name.into(),
                    value,
                    secret: false,
                };
                Value::Record {
                    def,
                    life: None,
                    name: "Datagram".into(),
                    fields: vec![
                        field("data", Value::Bytes(data.into())),
                        field("from", self.str_value(&from)),
                    ],
                }
            }
            _ => return Err(self.panic(format!("a socket has no method `{name}`"), span)),
        })
    }
}

/// The texts in a `[String]` value.
fn strings(v: Option<&Value<'_>>) -> Vec<String> {
    match v {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|v| match v {
                Value::Str(s) => Some(s.to_string()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

impl<'a> Interp<'a> {
    /// The arguments of `net.connect_tls` or `net.listen_tls` in parameter
    /// order, named ones in their place and missing ones as their default.
    pub(crate) fn tls_args(&self, name: &str, args: crate::calls::Args<'a>) -> Vec<Value<'a>> {
        let names: &[&str] = if name == "listen_tls" {
            &["address", "cert", "key", "protocols", "clients"]
        } else {
            &["address", "protocols", "roots", "cert", "key"]
        };
        crate::json::arrange(args, names)
            .into_iter()
            .zip(names)
            .map(|(v, n)| match v {
                Some(v) => v,
                None if *n == "protocols" => Value::List(std::rc::Rc::new(Vec::new())),
                None => self.str_value(""),
            })
            .collect()
    }
}
