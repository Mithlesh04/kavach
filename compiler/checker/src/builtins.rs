// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Prelude functions (spec 10.2); the methods of built-in types are in
//! `builtin_methods.rs`.

use crate::checker::Checker;
use crate::ty::{Prim, Ty};
use kavach_ast::{Arg, Span, TypeExpr};

pub(crate) struct MethodSig {
    pub(crate) params: Vec<Ty>,
    pub(crate) ret: Ty,
    /// Parameters after `params` that have a default, given by name.
    pub(crate) named: Vec<(&'static str, Ty)>,
}

/// The capability each impure prelude function needs (spec 8.1).
const IMPURE: &[(&str, &str)] = &[
    ("print", "Console"),
    ("ask", "Ai"),
    ("env", "Env"),
    ("random_bits", "Random"),
    ("random_ints", "Random"),
];

impl Checker<'_> {
    pub(crate) fn builtin_call(
        &mut self,
        name: &str,
        generics: &[TypeExpr],
        args: &[Arg],
        span: Span,
        expected: Option<&Ty>,
    ) -> Ty {
        if let Some((_, cap)) = IMPURE.iter().find(|(f, _)| *f == name) {
            self.note_cap(cap, span);
        }
        if let (true, Some((_, cap))) = (self.in_contract, IMPURE.iter().find(|(f, _)| *f == name))
        {
            self.error(span, format!("a contract can't call `{name}`: it uses {cap}, and contracts must be pure (spec 7.5)"));
        }
        if let Some(t) = self.sized_call(name, generics, args, span, expected) {
            return t;
        }
        if let Some(t) = self.int_type_call(name, generics, args, span) {
            return t;
        }
        let string = Ty::Prim(Prim::String);
        let one = |c: &mut Self, ty: Option<Ty>| -> Ty {
            if args.len() != 1 {
                c.error(span, format!("`{name}` takes one argument"));
            }
            match (args.first(), ty) {
                (Some(a), Some(t)) => c.check_expect(&a.value, &t),
                (Some(a), None) => c.value(&a.value, None),
                (None, _) => Ty::Error,
            }
        };
        match name {
            // `sum(amount)` in a `limit` rule: a total of the same type.
            "sum" => one(self, None),
            "print" | "debug" | "panic" => {
                let t = one(self, (name == "panic").then(|| string.clone()));
                if let Some(a) = args.first() {
                    self.check_secret(&a.value, name);
                }
                let _ = t;
                if name == "panic" { Ty::Never } else { Ty::Unit }
            }
            "ask" => self.ask_type(generics, args, span, false),
            // Its value is a secret (spec 10.13): keys come this way.
            "env" => {
                self.secret.insert((span.start, span.end));
                one(self, Some(string.clone()));
                Ty::Result(Box::new(string), Box::new(self.error_ty()))
            }
            "reveal" => one(self, None),
            // `const s: Shared<Node?> = Shared(node)`: the wanted type's.
            "Shared" => {
                let want = match expected.map(|t| self.inf.shallow(t)) {
                    Some(Ty::Builtin("Shared", args)) => args.first().cloned(),
                    _ => None,
                };
                let t = one(self, want.clone());
                Ty::Builtin("Shared", vec![want.unwrap_or(t)])
            }
            // `Channel<T>(capacity: n)`: its two ends (spec 9.4).
            "Channel" => {
                let t = match generics.first() {
                    Some(g) => self.lower(g, None),
                    None => {
                        self.error(
                            span,
                            "write the type of the values: `Channel<T>(capacity: n)`",
                        );
                        Ty::Error
                    }
                };
                match args {
                    [a] if a.name.as_ref().is_some_and(|n| n.name == "capacity") => {
                        self.check_expect(&a.value, &Ty::Prim(Prim::Int));
                    }
                    _ => self.error(
                        span,
                        "a channel takes its capacity: `Channel<T>(capacity: n)`",
                    ),
                }
                Ty::Tuple(vec![
                    Ty::Builtin("Sender", vec![t.clone()]),
                    Ty::Builtin("Receiver", vec![t]),
                ])
            }
            "alloc" => {
                self.need_unsafe("`alloc`", span);
                one(self, Some(Ty::Prim(Prim::Int)));
                match expected.map(|t| self.inf.shallow(t)) {
                    Some(t @ Ty::Builtin("Ptr", _)) => t,
                    _ => Ty::Builtin("Ptr", vec![self.fresh()]),
                }
            }
            "free" => {
                self.need_unsafe("`free`", span);
                one(self, None);
                Ty::Unit
            }
            // `std.log`'s way out (spec 14.12): a level, a line, and the
            // event (its type and module choose its lowest level), written
            // as `kavach.toml` says. Programs log typed events instead.
            "log_write" => {
                let module = &self.res.modules[self.res.module_at(span.start)].name;
                if !module.starts_with("std.") {
                    self.error(
                        span,
                        "`log_write` is for `std.log`: log an event with `log.info(event)` (spec 14.12)",
                    );
                }
                let int = Ty::Prim(Prim::Int);
                match args {
                    [level, line, event] => {
                        self.check_expect(&level.value, &int);
                        self.check_expect(&line.value, &string);
                        self.value(&event.value, None);
                    }
                    _ => self.error(span, "`log_write` takes a level, a line and the event"),
                }
                Ty::Unit
            }
            // `std.test`'s source of `Gen` choices (spec 14.7): a number
            // from 0 to `span`, from the test run's seeded generator.
            "test_random" => {
                let module = &self.res.modules[self.res.module_at(span.start)].name;
                if !module.starts_with("std.") {
                    self.error(
                        span,
                        "`test_random` is for `std.test`: make values from a `Gen` (spec 14.7)",
                    );
                }
                let int = Ty::Prim(Prim::Int);
                match args {
                    [n] => {
                        self.check_expect(&n.value, &int);
                    }
                    _ => self.error(span, "`test_random` takes the largest number it may give"),
                }
                int
            }
            // `std.random`'s source of 64 random bits (spec 14.15).
            "random_bits" => {
                let module = &self.res.modules[self.res.module_at(span.start)].name;
                if !module.starts_with("std.") {
                    self.error(
                        span,
                        "`random_bits` is for `std.random`: use `random.int(from, until)` and the others (spec 14.15)",
                    );
                }
                if !args.is_empty() {
                    self.error(span, "`random_bits` takes no arguments");
                }
                Ty::Prim(Prim::Int)
            }
            // `tensor.random_int`'s numbers, many at once (spec 14.33).
            "random_ints" => {
                let module = &self.res.modules[self.res.module_at(span.start)].name;
                if !module.starts_with("std.") {
                    self.error(
                        span,
                        "`random_ints` is for `std.tensor`: use `tensor.random_int(shape, from, until)` (spec 14.33)",
                    );
                }
                let int = Ty::Prim(Prim::Int);
                match args {
                    [n, from, until] => {
                        for a in [n, from, until] {
                            self.check_expect(&a.value, &int);
                        }
                    }
                    _ => self.error(span, "`random_ints` takes how many, `from` and `until`"),
                }
                Ty::List(Box::new(int))
            }
            // `Bytes([104, 105])`: bytes from a list of `UInt8` (spec 2.13).
            "Bytes" => {
                one(self, Some(Ty::List(Box::new(Ty::Prim(Prim::UInt8)))));
                self.bytes_of_literal(args)
            }
            // `TextBuilder()`, `TextBuilder(text)` (spec 2.18).
            "TextBuilder" => {
                if !args.is_empty() {
                    let found = one(self, None);
                    let found = self.inf.resolve(&found);
                    let string = Ty::Prim(Prim::String);
                    if !crate::text_views::is_text(&found) && !self.coerce(&found, &string) {
                        self.mismatch(&args[0].value, &string, &found);
                    }
                }
                Ty::Builtin("TextBuilder", Vec::new())
            }
            // `Error("disk full")`: an error that is only a message (spec 5.5).
            "Error" => {
                one(self, Some(Ty::Prim(Prim::String)));
                self.error_ty()
            }
            _ => match Prim::from_name(name) {
                // `Float(n)`, `Int(text)`: conversions (spec 2.1).
                Some(p) => self.convert(p, args, span),
                None => {
                    for a in args {
                        self.value(&a.value, None);
                    }
                    let how = match name {
                        "Set" => {
                            ": make a set with `Set.of(1, 2, 3)`, or `Set.capacity(n)` and `add`"
                        }
                        "Dict" => ": make a dict with `{\"a\": 1}`, or `Dict.capacity(n)`",
                        _ => "",
                    };
                    self.error(
                        span,
                        format!("`{name}` can't be called like a function{how} (spec 2.2)"),
                    );
                    Ty::Error
                }
            },
        }
    }

    pub(crate) fn error_ty(&self) -> Ty {
        Ty::Builtin("Error", Vec::new())
    }
}
