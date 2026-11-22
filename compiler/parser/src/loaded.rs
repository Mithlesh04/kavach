// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! `extern c library Name { fn … }` (spec 10.7): the functions of a C
//! library the program loads while running. The block becomes the type
//! `Name`: a record holding each function's address (a field no program
//! can name), with `Name.open(path)` and one method per function.

use crate::parser::Parser;
use kavach_ast::*;

/// The field of a loaded library holding its functions' addresses; the
/// `-` keeps programs from naming it, and fits the runtime's shape table.
pub const ADDRESSES: &str = "c-functions";

impl Parser<'_> {
    /// The type of `extern c library Name { … }`; `word` is where
    /// `library` is written.
    pub(crate) fn loaded_library(
        &mut self,
        start: Span,
        word: Span,
        docs: Vec<String>,
        is_pub: bool,
    ) -> Item {
        let name = self.ident("the library's type name, as in `extern c library Adder`");
        let span = name.span;
        // Each name made here gets a place of its own, a letter of
        // `library`, as names are told apart by where they are.
        let at = |k: u32| Span {
            start: word.start + k,
            end: word.start + k + 1,
        };
        let ident = |n: &str, k: u32| Ident {
            name: n.to_string(),
            span: at(k),
        };
        let named = |n: &str, k: u32| TypeExpr {
            kind: TypeKind::Named {
                path: Path {
                    segments: vec![ident(n, k)],
                    span: at(k),
                },
                args: Vec::new(),
            },
            span: at(k),
        };
        let mut methods = self.methods(false);
        let mut number = 0;
        for item in &mut methods {
            item.is_pub = item.is_pub || is_pub;
            if let ItemKind::Fn(f) = &mut item.kind {
                if f.body.is_some() {
                    self.error(
                        f.name.span,
                        "a C function has no body here: its code is in the library",
                    );
                }
                // `self` inside the function's name, a place no other
                // name starts at.
                let inside = Span {
                    start: f.name.span.start + 1,
                    end: f.name.span.start + 1,
                };
                f.params.insert(0, self_param(inside));
                f.c_loaded = Some(CLoaded::Function(number));
                number += 1;
            }
        }
        // `fn open(path: String) -> Name ! LibraryError`, its body made
        // when it is compiled.
        let mut open = self.bare_fn(ident("open", 0));
        open.params.push(Param {
            name: ident("path", 1),
            ty: Some(named("String", 2)),
            default: None,
            is_secret: false,
            is_inout: false,
        });
        open.ret = Some(named(&name.name, 3));
        open.errors = Some(named("LibraryError", 4));
        open.c_loaded = Some(CLoaded::Open);
        methods.insert(
            0,
            Item {
                kind: ItemKind::Fn(open),
                docs: Vec::new(),
                is_pub: true,
                is_out: false,
                span,
            },
        );
        let addresses = Field {
            name: ident(ADDRESSES, 5),
            ty: TypeExpr {
                kind: TypeKind::List(Box::new(named("Int", 6))),
                span: at(6),
            },
            default: None,
            is_pub: false,
            is_read: false,
            is_secret: false,
            docs: Vec::new(),
        };
        let body = RecordBody {
            fields: vec![addresses],
            methods,
            invariants: Vec::new(),
            distinct: false,
        };
        Item {
            kind: ItemKind::Type(TypeDecl {
                name,
                generics: Vec::new(),
                implements: Vec::new(),
                body: TypeBody::Record(body),
            }),
            docs,
            is_pub,
            is_out: false,
            span: start,
        }
    }
}

fn self_param(span: Span) -> Param {
    Param {
        name: Ident {
            name: "self".to_string(),
            span,
        },
        ty: None,
        default: None,
        is_secret: false,
        is_inout: false,
    }
}
