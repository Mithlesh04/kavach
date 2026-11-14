//! More methods of built-in types (spec 14.2): what text, lists, maps
//! and numbers have beyond their own methods.

use crate::builtins::MethodSig;
use crate::checker::Checker;
use crate::ty::{Prim, Ty};

use crate::stdlib::number;

impl Checker<'_> {
    /// `b.as_list<T>()` (spec 2.13): `[T]?`, for a number of fixed size.
    pub(crate) fn bytes_as_list(
        &mut self,
        generics: &[kavach_ast::TypeExpr],
        args: &[kavach_ast::Arg],
        span: kavach_ast::Span,
    ) -> Ty {
        if !args.is_empty() {
            self.error(span, "`as_list` takes no arguments: `b.as_list<Float32>()`");
        }
        match self.fixed_number(generics, span, "as_list", "b.as_list<Float32>()") {
            Ty::Error => Ty::Error,
            item => Ty::Optional(Box::new(Ty::List(Box::new(item)))),
        }
    }

    /// `b.read<T>(at, big_endian: false)` (spec 2.13): `T?`.
    pub(crate) fn bytes_read(
        &mut self,
        generics: &[kavach_ast::TypeExpr],
        args: &[kavach_ast::Arg],
        span: kavach_ast::Span,
    ) -> Ty {
        let item = self.fixed_number(generics, span, "read", "b.read<UInt32>(at)");
        let params = [
            ("at".to_string(), Ty::Prim(Prim::Int), false),
            ("big_endian".to_string(), Ty::Prim(Prim::Bool), true),
        ];
        self.args(args, &params, span, "`read`", "argument");
        match item {
            Ty::Error => Ty::Error,
            item => Ty::Optional(Box::new(item)),
        }
    }

    /// `Bytes.of<T>(n, big_endian: false)` (spec 2.13): `Bytes`.
    pub(crate) fn bytes_of(
        &mut self,
        generics: &[kavach_ast::TypeExpr],
        args: &[kavach_ast::Arg],
        span: kavach_ast::Span,
    ) -> Ty {
        let item = self.fixed_number(generics, span, "Bytes.of", "Bytes.of<UInt32>(n)");
        let params = [
            ("n".to_string(), item, false),
            ("big_endian".to_string(), Ty::Prim(Prim::Bool), true),
        ];
        self.args(args, &params, span, "`Bytes.of`", "argument");
        Ty::Prim(Prim::Bytes)
    }

    /// The one type argument of `as_list`, `read` or `of` (`name`): a
    /// number of fixed size, or `Ty::Error` after saying why. `how`
    /// shows a call.
    fn fixed_number(
        &mut self,
        generics: &[kavach_ast::TypeExpr],
        span: kavach_ast::Span,
        name: &str,
        how: &str,
    ) -> Ty {
        let item = match generics {
            [g] => self.lower(g, None),
            _ => {
                self.error(span, format!("write the type of the numbers: `{how}`"));
                return Ty::Error;
            }
        };
        let fixed = matches!(&item, Ty::Prim(p) if p.is_int())
            || matches!(item, Ty::Prim(Prim::Float | Prim::Float32));
        if !fixed && !matches!(item, Ty::Error) {
            self.error(
                span,
                format!("`{name}` reads numbers of a fixed size: `Int8`, `Int16`, `Int32`, `Int`, `UInt8`, `UInt16`, `UInt32`, `UInt64`, `Float32` or `Float`"),
            );
            return Ty::Error;
        }
        item
    }

    /// More methods of built-in types (spec 14.2).
    pub(crate) fn more_methods(&mut self, recv: &Ty, name: &str) -> Option<MethodSig> {
        let (int, boolean, string) = (
            Ty::Prim(Prim::Int),
            Ty::Prim(Prim::Bool),
            Ty::Prim(Prim::String),
        );
        let sig = |params: Vec<Ty>, ret: Ty| {
            Some(MethodSig {
                params,
                ret,
                named: Vec::new(),
            })
        };
        let list = |t: Ty| Ty::List(Box::new(t));
        let opt = |t: Ty| Ty::Optional(Box::new(t));
        let f = |p: Ty, r: Ty| Ty::Fn(vec![p], Box::new(r), None);
        match recv {
            t if number(t) => match name {
                "abs" => sig(vec![], t.clone()),
                "bits" if *t == Ty::Prim(Prim::Float) => sig(vec![], Ty::Prim(Prim::UInt64)),
                "near" if matches!(t, Ty::Prim(Prim::Float | Prim::Float32)) => {
                    sig(vec![t.clone()], Ty::Prim(Prim::Bool))
                }
                "min" | "max" => sig(vec![t.clone()], t.clone()),
                "clamp" => sig(vec![t.clone(), t.clone()], t.clone()),
                // Decision 0051.
                "div_floor" | "mod" if matches!(t, Ty::Prim(p) if p.is_int()) => {
                    sig(vec![t.clone()], t.clone())
                }
                "round" | "round_even" if *t == Ty::Prim(Prim::Decimal) => {
                    sig(vec![Ty::Prim(Prim::Int)], t.clone())
                }
                _ => None,
            },
            Ty::Prim(Prim::String) => match name {
                "replace" => sig(vec![string.clone(), string.clone()], string),
                "lines" => sig(vec![], list(string)),
                "chars" => sig(vec![], list(Ty::Prim(Prim::Char))),
                "index_of" => sig(vec![string], opt(int)),
                "slice" => sig(vec![int.clone(), int], string),
                _ => None,
            },
            Ty::List(t) => {
                let t = (**t).clone();
                let item = self.inf.resolve(&t);
                match name {
                    // `[Char]` joins too: `s.chars().reverse().join("")` (spec 14.2).
                    "join" if matches!(item, Ty::Prim(Prim::Char)) => {
                        sig(vec![string.clone()], string)
                    }
                    "join" if self.inf.unify(&t, &string) => sig(vec![string.clone()], string),
                    "sort" | "min" | "max" if self.ordered(&item) == Some(false) => None,
                    "sort" => sig(vec![], list(t)),
                    "min" | "max" => sig(vec![], opt(t)),
                    "sort_by" => {
                        let key = self.fresh();
                        sig(vec![f(t.clone(), key)], list(t))
                    }
                    "reverse" => sig(vec![], list(t)),
                    "take" | "skip" => sig(vec![int], list(t)),
                    "find" => sig(vec![f(t.clone(), boolean)], opt(t)),
                    "count" => sig(vec![f(t, boolean)], int),
                    "index_of" => sig(vec![t], opt(int)),
                    _ => None,
                }
            }
            Ty::Builtin("Regex", _) => {
                let opt_list = |t: Ty| opt(list(t));
                match name {
                    "is_match" => sig(vec![string], boolean),
                    "find" => sig(vec![string.clone()], opt(string)),
                    "find_all" | "split" => sig(vec![string.clone()], list(string)),
                    "captures" => sig(vec![string.clone()], opt_list(opt(string))),
                    "named_captures" => sig(
                        vec![string.clone()],
                        opt(Ty::Map(Box::new(string.clone()), Box::new(string))),
                    ),
                    "replace" | "replace_all" => sig(vec![string.clone(), string.clone()], string),
                    _ => None,
                }
            }
            Ty::Set(t) => {
                let t = (**t).clone();
                match name {
                    "add" | "remove" => sig(vec![t], Ty::Unit),
                    "contains" => sig(vec![t], boolean),
                    "len" => sig(vec![], int),
                    "is_empty" => sig(vec![], boolean),
                    "to_list" => sig(vec![], list(t)),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}
