// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Methods of built-in types (spec 14.2), and `x.timeout(d)` (spec 9.6).

use crate::builtins::MethodSig;
use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::{Arg, Expr, ExprKind, Span};

impl Checker<'_> {
    /// Methods of built-in types, with fresh types for generic results.
    pub(crate) fn builtin_method(&mut self, recv: &Ty, name: &str) -> Option<MethodSig> {
        self.core_method(recv, name)
            .or_else(|| self.more_methods(recv, name))
            .or_else(|| self.enum_method(recv, name))
            .or_else(|| self.view_method(recv, name))
    }

    /// `x.timeout(d)` (spec 9.6), where `x` is `task.wait()`,
    /// `receive.take()` or `send.put(v)`: `x`'s value, or an error when
    /// `d` passes first. `None` when `x` is none of those.
    pub(crate) fn timeout(&mut self, x: &Expr, args: &[Arg], span: Span) -> Option<Ty> {
        let ExprKind::Call { callee, .. } = &x.kind else {
            return None;
        };
        let ExprKind::Field { base, name, .. } = &callee.kind else {
            return None;
        };
        let net = matches!(
            name.name.as_str(),
            "accept"
                | "read"
                | "write"
                | "send"
                | "receive"
                | "read_stream"
                | "write_stream"
                | "connect"
                | "connect_tls"
                | "connect_quic"
                | "start_tls"
                | "accept_tls"
        );
        if !net && !matches!(name.name.as_str(), "wait" | "take" | "put") {
            return None;
        }
        // `net.connect(address).timeout(d)` (spec 14.10).
        let from_net = matches!(&base.kind, ExprKind::Name(id)
            if self.res.uses.get(&id.span.start)
                .and_then(|d| self.std_modules.get(d))
                .is_some_and(|m| m == "net"));
        let inferred = self.infer(x, None);
        let ok = match inferred.clone() {
            Ty::Result(ok, _) => *ok,
            t => t,
        };
        // `infer` typed the receiver too.
        let recv = self
            .expr_types
            .iter()
            .rev()
            .find(|(sp, _)| *sp == base.span)
            .map(|(_, t)| t.clone())
            .unwrap_or(Ty::Error);
        let recv = self.inf.resolve(&recv);
        let fits = matches!(
            (&recv, name.name.as_str()),
            (Ty::Builtin("Task", _), "wait")
                | (Ty::Builtin("Receiver", _), "take")
                | (Ty::Builtin("Sender", _), "put")
                | (
                    Ty::Builtin("Listener" | "Connection" | "UdpSocket", _),
                    "accept"
                        | "read"
                        | "write"
                        | "send"
                        | "receive"
                        | "read_stream"
                        | "write_stream"
                        | "start_tls"
                        | "accept_tls"
                )
        ) || (from_net
            && matches!(
                name.name.as_str(),
                "connect" | "connect_tls" | "connect_quic"
            ));
        if !fits {
            return None;
        }
        let duration = self.builtin_ty("Duration");
        match args {
            [a] if a.name.is_none() => {
                self.check_expect(&a.value, &duration);
            }
            _ => self.error(
                span,
                "`timeout` takes how long to wait: `.timeout(5.seconds)`",
            ),
        }
        // A network wait fails with its own `NetError.TimedOut`.
        if net {
            return Some(inferred);
        }
        Some(Ty::Result(Box::new(ok), Box::new(self.error_ty())))
    }

    fn core_method(&mut self, recv: &Ty, name: &str) -> Option<MethodSig> {
        let int = Ty::Prim(Prim::Int);
        let boolean = Ty::Prim(Prim::Bool);
        let string = Ty::Prim(Prim::String);
        let sig = |params: Vec<Ty>, ret: Ty| {
            Some(MethodSig {
                params,
                ret,
                named: Vec::new(),
            })
        };
        let f = |p: Ty, r: Ty| Ty::Fn(vec![p], Box::new(r), None);
        let list = |t: Ty| Ty::List(Box::new(t));
        let opt = |t: Ty| Ty::Optional(Box::new(t));
        match recv {
            Ty::List(t) => {
                let t = (**t).clone();
                let u = self.fresh();
                match name {
                    "len" => sig(vec![], int),
                    "is_empty" => sig(vec![], boolean),
                    "push" => sig(vec![t], Ty::Unit),
                    // Made with `List.capacity(n)` and holding `n` (spec 2.2).
                    "is_full" => sig(vec![], boolean),
                    "get" => sig(vec![int], opt(t)),
                    "first" | "last" => sig(vec![], opt(t.clone())),
                    "pop" => sig(vec![], opt(t)),
                    // Swaps two items in place (spec 2.2).
                    "swap" => sig(vec![int.clone(), int], Ty::Unit),
                    "contains" => sig(vec![t], boolean),
                    "filter" => sig(vec![f(t.clone(), boolean)], list(t)),
                    "any" | "all" => sig(vec![f(t, boolean.clone())], boolean),
                    // Runs `f` on every item, in order, for what it does.
                    "for_each" => sig(vec![f(t, u)], Ty::Unit),
                    "find_index" | "find_last_index" => sig(vec![f(t, boolean)], opt(int)),
                    "map" => sig(vec![f(t, u.clone())], list(u)),
                    "flat_map" => sig(vec![f(t, list(u.clone()))], list(u)),
                    "sum" => sig(vec![f(t, u.clone())], u),
                    "enumerate" => sig(vec![], list(Ty::Tuple(vec![int.clone(), t]))),
                    // Each item in a task of its own, at most `max` at a
                    // time (0: no limit), results in order (spec 9.3).
                    "parallel_map" => Some(MethodSig {
                        params: vec![f(t, u.clone())],
                        ret: list(u),
                        named: vec![("max", int)],
                    }),
                    _ => None,
                }
            }
            Ty::Map(k, v) => {
                let (k, v) = ((**k).clone(), (**v).clone());
                match name {
                    "len" => sig(vec![], int),
                    "is_empty" => sig(vec![], boolean),
                    "get" => sig(vec![k], opt(v)),
                    "contains_key" => sig(vec![k.clone()], boolean),
                    "is_full" => sig(vec![], boolean),
                    "remove" => sig(vec![k], Ty::Unit),
                    "keys" => sig(vec![], list(k)),
                    "values" => sig(vec![], list(v)),
                    _ => None,
                }
            }
            Ty::Prim(Prim::String) => match name {
                "len" => sig(vec![], int),
                "is_empty" => sig(vec![], boolean),
                "upper" | "lower" | "trim" | "trim_start" | "trim_end" | "capitalize" | "title" => {
                    sig(vec![], string)
                }
                "contains" | "starts_with" | "ends_with" => sig(vec![string], boolean),
                "is_blank" => sig(vec![], boolean),
                // `Comparable` (spec 2.16): below 0 when `s` comes first.
                "compare" => sig(vec![string.clone()], int.clone()),
                "words" => sig(vec![], list(string.clone())),
                "last_index_of" => sig(vec![string], opt(int)),
                "count" => sig(vec![string], int),
                "remove_prefix" | "remove_suffix" => sig(vec![string.clone()], string),
                // `fill` is a `Char`, a space when not given (spec 14.2).
                "pad_left" | "pad_right" => Some(MethodSig {
                    params: vec![int],
                    ret: string,
                    named: vec![("fill", Ty::Prim(Prim::Char))],
                }),
                "split" => sig(vec![string.clone()], list(string)),
                "repeat" => sig(vec![int], string),
                "bytes" => sig(vec![], Ty::Prim(Prim::Bytes)),
                _ => None,
            },
            // Spec 14.2, *Characters*.
            Ty::Prim(Prim::Char) => match name {
                "is_digit" | "is_letter" | "is_alphanumeric" | "is_whitespace" | "is_upper"
                | "is_lower" => sig(vec![], boolean),
                _ => None,
            },
            // Spec 2.13.
            Ty::Prim(Prim::Bytes) => {
                let byte = Ty::Prim(Prim::UInt8);
                let bytes = Ty::Prim(Prim::Bytes);
                match name {
                    "len" => sig(vec![], int),
                    "is_empty" => sig(vec![], Ty::Prim(Prim::Bool)),
                    "get" => sig(vec![int], opt(byte)),
                    "index_of" => sig(vec![bytes], opt(int)),
                    // `b.text(start, end)`: one copy of a range (2.13).
                    "text" => Some(MethodSig {
                        params: vec![],
                        ret: opt(string),
                        named: vec![("start", int.clone()), ("end", int.clone())],
                    }),
                    "to_list" => sig(vec![], list(byte)),
                    "crc32c" => sig(vec![], int),
                    "xxhash64" => sig(vec![], Ty::Prim(Prim::UInt64)),
                    _ => None,
                }
            }
            Ty::Prim(p) if p.is_int() => {
                let t = Ty::Prim(*p);
                match name {
                    // Wrapping has no meaning for a range (spec 2.1).
                    "wrapping_add" | "wrapping_sub" | "wrapping_mul"
                        if !matches!(p, Prim::Ranged(..)) =>
                    {
                        sig(vec![t.clone()], t)
                    }
                    "checked_add" | "checked_sub" | "checked_mul" | "checked_div" => {
                        sig(vec![t.clone()], opt(t))
                    }
                    _ => None,
                }
            }
            Ty::Optional(t) => match name {
                "unwrap" => sig(vec![], (**t).clone()),
                "is_none" | "is_some" => sig(vec![], boolean),
                _ => None,
            },
            Ty::Builtin("Shared", args) => {
                let t = args.first().cloned().unwrap_or(Ty::Error);
                match name {
                    "get" => sig(vec![], t),
                    // Both give a result found under the lock (spec 6.2).
                    "update" => sig(vec![f(t.clone(), t.clone())], t),
                    "update_with" => {
                        let r = self.fresh();
                        sig(vec![f(t.clone(), Ty::Tuple(vec![t, r.clone()]))], r)
                    }
                    "weak" => sig(vec![], Ty::Builtin("Weak", vec![t])),
                    _ => None,
                }
            }
            // The last `n` items pushed, oldest first (spec 2.2).
            Ty::Builtin("Ring", args) => {
                let t = args.first().cloned().unwrap_or(Ty::Error);
                match name {
                    "push" => sig(vec![t], Ty::Unit),
                    "len" => sig(vec![], int),
                    "is_empty" | "is_full" => sig(vec![], boolean),
                    "first" | "last" => sig(vec![], opt(t)),
                    "get" => sig(vec![int], opt(t)),
                    "contains" => sig(vec![t], boolean),
                    "to_list" => sig(vec![], list(t)),
                    _ => None,
                }
            }
            // A back-link that doesn't keep its `Shared` alive (spec 6.2).
            Ty::Builtin("Weak", args) if name == "shared" => {
                sig(vec![], opt(Ty::Builtin("Shared", args.clone())))
            }
            // Raw memory (spec 6.4).
            Ty::Builtin("Ptr", args) => {
                let t = args.first().cloned().unwrap_or(Ty::Error);
                match name {
                    "read" => sig(vec![int], t),
                    "write" => sig(vec![int, t.clone()], Ty::Unit),
                    // Registers (spec 6.5): never merged, dropped or moved.
                    "read_volatile" => sig(vec![], t),
                    "write_volatile" => sig(vec![t], Ty::Unit),
                    "address" => sig(vec![], int),
                    "text" if t == Ty::Prim(Prim::UInt8) => sig(vec![], opt(string)),
                    // `n` bytes copied out (spec 6.4).
                    "bytes" if t == Ty::Prim(Prim::UInt8) => sig(vec![int], Ty::Prim(Prim::Bytes)),
                    _ => None,
                }
            }
            Ty::Builtin("Sender", args) => match name {
                "put" => sig(vec![args.first().cloned().unwrap_or(Ty::Error)], Ty::Unit),
                // Never waits: whether there was room (spec 9.4).
                "try_put" => sig(vec![args.first().cloned().unwrap_or(Ty::Error)], boolean),
                "close" => sig(vec![], Ty::Unit),
                _ => None,
            },
            Ty::Builtin("Receiver", args) => match name {
                // `Taken<T>` (spec 9.4, decision 0097).
                "take" => {
                    let item = args.first().cloned().unwrap_or(Ty::Error);
                    let taken = match self.res.std_type("Taken") {
                        Some(def) => Ty::Adt(def, vec![item]),
                        None => Ty::Error,
                    };
                    sig(vec![], taken)
                }
                _ => None,
            },
            // `std.net` (spec 14.10).
            Ty::Builtin("Listener", _) => {
                let fails = |t: Ty| Ty::Result(Box::new(t), Box::new(self.builtin_ty("NetError")));
                match name {
                    "accept" => sig(vec![], fails(Ty::Builtin("Connection", Vec::new()))),
                    "address" => sig(vec![], string),
                    "close" => sig(vec![], Ty::Unit),
                    _ => None,
                }
            }
            Ty::Builtin("UdpSocket", _) => {
                let fails = |t: Ty| Ty::Result(Box::new(t), Box::new(self.builtin_ty("NetError")));
                match name {
                    "send" => sig(vec![Ty::Prim(Prim::Bytes), string], fails(Ty::Unit)),
                    "receive" => sig(vec![], fails(self.builtin_ty("Datagram"))),
                    "address" => sig(vec![], string),
                    "close" => sig(vec![], Ty::Unit),
                    _ => None,
                }
            }
            Ty::Builtin("Connection", _) => {
                let fails = |t: Ty| Ty::Result(Box::new(t), Box::new(self.builtin_ty("NetError")));
                let bytes = Ty::Prim(Prim::Bytes);
                match name {
                    "read" => sig(vec![], fails(opt(bytes.clone()))),
                    "write" => sig(vec![bytes.clone()], fails(Ty::Unit)),
                    "protocol" => sig(vec![], opt(Ty::Prim(Prim::String))),
                    // The other side's address (spec 14.10).
                    "peer" => sig(vec![], Ty::Prim(Prim::String)),
                    // A client's certificate, with mutual TLS (decision 0099).
                    "client_cert" => sig(vec![], opt(self.builtin_ty("ClientCert"))),
                    // The other side's certificate, as DER (spec 14.10).
                    "peer_certificate" => sig(vec![], opt(bytes.clone())),
                    // TLS begun on a plain connection (spec 14.10).
                    "start_tls" | "accept_tls" => {
                        let text = Ty::Prim(Prim::String);
                        let texts = Ty::List(Box::new(text.clone()));
                        let (params, named) = match name {
                            "start_tls" => (
                                vec![text.clone()],
                                vec![
                                    ("protocols", texts),
                                    ("roots", text.clone()),
                                    ("cert", text.clone()),
                                    ("key", text),
                                ],
                            ),
                            _ => (
                                vec![text.clone(), text.clone()],
                                vec![("protocols", texts), ("clients", text)],
                            ),
                        };
                        Some(MethodSig {
                            params,
                            ret: fails(Ty::Builtin("Connection", Vec::new())),
                            named,
                        })
                    }
                    // QUIC streams (spec 14.10).
                    "open_stream" => sig(vec![Ty::Prim(Prim::Bool)], fails(Ty::Prim(Prim::Int))),
                    "write_stream" => sig(
                        vec![Ty::Prim(Prim::Int), bytes.clone(), Ty::Prim(Prim::Bool)],
                        fails(Ty::Unit),
                    ),
                    "read_stream" => sig(vec![Ty::Prim(Prim::Int)], fails(opt(bytes.clone()))),
                    "close" => sig(vec![], Ty::Unit),
                    _ => None,
                }
            }
            Ty::Builtin("Task", args) => match name {
                // Stops the task at once (spec 9.6, decision 0075).
                "cancel" => sig(vec![], Ty::Unit),
                "wait" => sig(
                    vec![],
                    Ty::Result(
                        Box::new(args.first().cloned().unwrap_or(Ty::Error)),
                        Box::new(self.error_ty()),
                    ),
                ),
                _ => None,
            },
            _ => None,
        }
    }
}
