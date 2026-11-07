//! Lowering the standard library (spec 14): calls to `std` modules, their
//! constants, more methods of built-in types, and sets.

use crate::ir::{self, BinOp, Block, ExprKind as K, Shape, Stmt, Type, UnOp};
use crate::lower::{Lowered, Lowerer};
use crate::rt_op::RtOp;
use kavach_ast::{self as ast, ItemKind};
use kavach_checker::Ty;
use kavach_resolver::DefId;
use std::collections::HashMap;

/// Names brought in by `use std.<module>`, and the module.
pub(crate) fn std_modules(
    c: &kavach_driver::Compiled,
    decl: &HashMap<u32, DefId>,
) -> HashMap<DefId, String> {
    let mut out = HashMap::new();
    for item in &c.file.items {
        let ItemKind::Use(u) = &item.kind else {
            continue;
        };
        let segs = &u.path.segments;
        if segs.len() == 2 && segs[0].name == "std" && u.names.is_empty() {
            let name = u.alias.as_ref().unwrap_or(&segs[1]);
            if let Some(&def) = decl.get(&name.span.start) {
                out.insert(def, segs[1].name.clone());
            }
        }
    }
    out
}

impl<'a> Lowerer<'a> {
    fn at(&self, kind: K, ty: Type, line: u32) -> ir::Expr {
        ir::Expr { kind, ty, line }
    }

    /// `math.pi`.
    pub(crate) fn std_constant(
        &self,
        module: &str,
        name: &str,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let line = self.line(span);
        match (module, name) {
            ("math", "pi") => Ok(self.at(K::Float(std::f64::consts::PI), Type::F64, line)),
            ("math", "e") => Ok(self.at(K::Float(std::f64::consts::E), Type::F64, line)),
            _ => Err(self.unsupported(format!("`{module}.{name}`"), span)),
        }
    }

    /// `fs.read_text(path)` and the other module functions (spec 14.3–14.6).
    pub(crate) fn std_call(
        &mut self,
        module: &str,
        name: &str,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let op = match (module, name) {
            ("fs", "read_text") => RtOp::FsReadText,
            ("fs", "write_text") => RtOp::FsWriteText,
            ("fs", "append_text") => RtOp::FsAppendText,
            ("fs", "exists") => RtOp::FsExists,
            ("fs", "read_bytes") => RtOp::FsReadBytes,
            ("fs", "lines") => RtOp::FsLines,
            ("net", "listen") => RtOp::NetListen,
            ("net", "connect") => RtOp::NetConnect,
            ("net", "connect_tls" | "listen_tls") => {
                return self.tls_call(name, args, ty, line, span);
            }
            ("net", "connect_quic") => RtOp::NetConnectQuic,
            ("net", "bind") => RtOp::NetBind,
            ("fs", "chunks") => RtOp::FsChunks,
            ("fs", "write_bytes") => RtOp::FsWriteBytes,
            ("fs", "write_durable") => RtOp::FsWriteDurable,
            ("fs", "list") => RtOp::FsList,
            ("fs", "remove") => RtOp::FsRemove,
            ("fs", "create_dir") => RtOp::FsCreateDir,
            ("time", "now_ms") => RtOp::TimeNowMs,
            ("time", "monotonic_ms") => RtOp::TimeMonotonicMs,
            ("time", "sleep_ms") => RtOp::TimeSleepMs,
            ("time", "zone_offset") => RtOp::ZoneOffset,
            ("agents", "call_key") => RtOp::JournalCallKey,
            ("agents", "request_fields") => RtOp::AgentApprovals,
            ("agents", "answer_request") => RtOp::AgentAnswer,
            ("clipboard", "read_text") => RtOp::ClipReadText,
            ("clipboard", "write_text") => RtOp::ClipWriteText,
            ("devices", "serial_ports") => RtOp::SerialPorts,
            ("devices", "open_serial_id") => RtOp::SerialOpen,
            ("devices", "read_serial") => RtOp::SerialRead,
            ("devices", "write_serial") => RtOp::SerialWrite,
            ("devices", "close_serial") => RtOp::SerialClose,
            ("math", "sqrt") => RtOp::MathSqrt,
            ("math", "pow") => RtOp::MathPow,
            ("math", "floor") => RtOp::MathFloor,
            ("math", "ceil") => RtOp::MathCeil,
            ("math", "round") => RtOp::MathRound,
            ("io", "read_line") => RtOp::IoReadLine,
            ("io", "read_hidden") => RtOp::IoReadHidden,
            ("io", "is_terminal") => RtOp::IoIsTerminal,
            ("io", "colors") => RtOp::IoColors,
            ("io", "write") => RtOp::IoWrite,
            ("cli", "args") => RtOp::CliArgs,
            ("cli", "exit") => RtOp::CliExit,
            ("regex", "new") => RtOp::RegexNew,
            ("regex", "advanced") => RtOp::RegexAdvanced,
            ("json", _) => return self.json_call(name, args, ty, line, span),
            ("time", "after") => return self.time_after(args, ty, line, span),
            // A `Duration` is its milliseconds (spec 14.4).
            ("time", "sleep" | "every") => {
                let d = args
                    .first()
                    .ok_or_else(|| self.unsupported(format!("`time.{name}()`"), span))?;
                let d = self.expr(&d.value)?;
                let ms = self.at(K::GetField(Box::new(d), 0), Type::I64, line);
                if name == "every" {
                    return Ok(ms);
                }
                return Ok(self.at(K::Rt(RtOp::TimeSleepMs, vec![ms]), Type::Unit, line));
            }
            // An interval is its period; `for` over it waits (lower_stmt.rs).
            ("time", "every_ms") => {
                let ms = args
                    .first()
                    .ok_or_else(|| self.unsupported("`every_ms()`", span))?;
                return self.expr(&ms.value);
            }
            _ => return Err(self.unsupported(format!("`{module}.{name}`"), span)),
        };
        let mut values = Vec::new();
        // Approvals in code (spec 14.16): the project's agents folder, or
        // empty text outside a project.
        if matches!(op, RtOp::AgentApprovals | RtOp::AgentAnswer) {
            let folder = self.c.agents_folder(false).unwrap_or_default();
            let text = self.str_type();
            values.push(self.at(K::Str(folder), text, line));
        }
        for a in args {
            values.push(self.expr(&a.value)?);
        }
        // An op needs at least one operand; a missing one is a 0 slot.
        if values.is_empty() {
            values.push(self.at(K::Int(0), Type::I64, line));
        }
        // `math.sqrt` of a `Float32` (spec 14.5): worked out as a `Float`,
        // then rounded once, as the interpreter does.
        if module == "math" && ty == Type::F32 {
            let wide = values
                .into_iter()
                .map(|v| self.at(K::Convert(Box::new(v), false), Type::F64, line))
                .collect();
            let r = self.at(K::Rt(op, wide), Type::F64, line);
            return Ok(self.at(K::Convert(Box::new(r), false), Type::F32, line));
        }
        Ok(self.at(K::Rt(op, values), ty, line))
    }

    /// `Set.of(a, b, …)`: stored like a map with no values.
    pub(crate) fn set_of(
        &mut self,
        args: &'a [ast::Arg],
        ty: Type,
        line: u32,
    ) -> Lowered<ir::Expr> {
        let Some(Shape::Set(item)) = self.shapes.shape(ty).cloned() else {
            return Err(self.unsupported("this set", ast::Span::new(0, 0)));
        };
        let mut pairs = Vec::new();
        for a in args {
            let v = self.expr(&a.value)?;
            pairs.push((self.coerce(v, item)?, self.nothing(line)));
        }
        Ok(self.at(K::Map(pairs), ty, line))
    }

    /// An expression with no value: the `Unit` "values" of a set.
    pub(crate) fn nothing(&self, line: u32) -> ir::Expr {
        self.at(K::Block(Block::default()), Type::Unit, line)
    }

    /// More methods of built-in types (spec 14.2), or `None` if `name`
    /// isn't one of them. `values` holds the receiver, then the arguments.
    pub(crate) fn more_method(
        &mut self,
        name: &str,
        mut values: Vec<ir::Expr>,
        ty: Type,
        line: u32,
    ) -> Option<Lowered<ir::Expr>> {
        let recv = values[0].ty;
        let shape = self.shapes.shape(recv).cloned();
        let numeric = recv.is_int() || recv.is_float() || matches!(shape, Some(Shape::Decimal));
        if numeric && matches!(name, "abs" | "min" | "max" | "clamp") {
            return Some(Ok(self.number_method(name, values, line)));
        }
        if recv.is_float() && name == "near" {
            // Passed as `Float`s (a `Float32` widens exactly), with a third
            // operand saying which tolerance (spec 14.2).
            let mut wide: Vec<ir::Expr> = values
                .into_iter()
                .map(|v| self.at(K::Convert(Box::new(v), false), Type::F64, line))
                .collect();
            wide.push(self.at(K::Int(i128::from(recv == Type::F32)), Type::I64, line));
            return Some(Ok(self.at(K::Rt(RtOp::FloatNear, wide), ty, line)));
        }
        if recv.is_int() && matches!(name, "div_floor" | "mod") {
            return Some(Ok(self.floor_method(name, values, line)));
        }
        if matches!(shape, Some(Shape::Decimal)) && matches!(name, "round" | "round_even") {
            values.push(self.at(K::Int(i128::from(name == "round_even")), Type::I64, line));
            return Some(Ok(self.at(K::Rt(RtOp::DecRound, values), ty, line)));
        }
        if recv == Type::F64 && name == "bits" {
            return Some(Ok(self.at(K::Rt(RtOp::FloatBits, values), ty, line)));
        }
        let op = match (shape, name) {
            (Some(Shape::Str), "replace") => RtOp::StrReplace,
            (Some(Shape::Str), "lines") => RtOp::StrLines,
            (Some(Shape::Str), "split") => RtOp::StrSplit,
            (Some(Shape::Str), "chars") => RtOp::StrChars,
            (Some(Shape::Str), "index_of") => RtOp::StrIndexOf,
            (Some(Shape::Str), "slice") => RtOp::StrSlice,
            (Some(Shape::List(_)), "join") => RtOp::ListJoin,
            (Some(Shape::List(_)), "sort") => RtOp::ListSort,
            (Some(Shape::List(_)), "reverse") => RtOp::ListReverse,
            (Some(Shape::List(_)), "take") => RtOp::ListTake,
            (Some(Shape::List(_)), "skip") => RtOp::ListSkip,
            (Some(Shape::List(_)), "index_of") => RtOp::ListIndexOf,
            (Some(Shape::List(_)), "min") => RtOp::ListMin,
            (Some(Shape::List(_)), "max") => RtOp::ListMax,
            (Some(Shape::Regex), "is_match") => RtOp::RegexIsMatch,
            (Some(Shape::Regex), "find") => RtOp::RegexFind,
            (Some(Shape::Regex), "find_all") => RtOp::RegexFindAll,
            (Some(Shape::Regex), "captures") => RtOp::RegexCaptures,
            (Some(Shape::Regex), "named_captures") => RtOp::RegexNamedCaptures,
            (Some(Shape::Regex), "replace") => RtOp::RegexReplace,
            (Some(Shape::Regex), "replace_all") => RtOp::RegexReplaceAll,
            (Some(Shape::Regex), "split") => RtOp::RegexSplit,
            (Some(Shape::Set(_)), "add") => {
                values.push(self.nothing(line));
                RtOp::MapInsert
            }
            (Some(Shape::Set(_)), "remove") => RtOp::MapRemove,
            (Some(Shape::Set(_)), "contains") => RtOp::MapContains,
            (Some(Shape::Set(_)), "len") => RtOp::MapLen,
            (Some(Shape::Set(_)), "is_empty") => RtOp::MapIsEmpty,
            (Some(Shape::Set(_)), "to_list") => RtOp::MapKeys,
            _ => return None,
        };
        // A method that changes the set gives back the changed set.
        let ty = if op.changes_receiver() { recv } else { ty };
        Some(Ok(self.at(K::Rt(op, values), ty, line)))
    }

    /// `abs`, `min`, `max` and `clamp`, written with compares, so they are
    /// as fast as hand-written code.
    fn number_method(&mut self, name: &str, values: Vec<ir::Expr>, line: u32) -> ir::Expr {
        let t = values[0].ty;
        let mut stmts = Vec::new();
        let locals: Vec<ir::Local> = values
            .into_iter()
            .map(|v| {
                let l = self.new_local(t);
                stmts.push(Stmt::Let(l, v));
                l
            })
            .collect();
        let get = |s: &Self, l: ir::Local| s.local_expr(l, line);
        // `a` when `a op b`, else `b`.
        let pick = |s: &Self, op: BinOp, a: ir::Local, b: ir::Local| {
            let cond = s.at(
                K::Binary(op, Box::new(get(s, a)), Box::new(get(s, b))),
                Type::Bool,
                line,
            );
            s.at(
                K::If(Box::new(cond), wrap_value(get(s, a)), wrap_value(get(s, b))),
                t,
                line,
            )
        };
        let value = match name {
            "abs" => {
                let zero = self.zero_of(t, line);
                let negated = self.at(K::Unary(UnOp::Neg, Box::new(get(self, locals[0]))), t, line);
                let cond = self.at(
                    K::Binary(BinOp::Lt, Box::new(get(self, locals[0])), Box::new(zero)),
                    Type::Bool,
                    line,
                );
                self.at(
                    K::If(
                        Box::new(cond),
                        wrap_value(negated),
                        wrap_value(get(self, locals[0])),
                    ),
                    t,
                    line,
                )
            }
            "min" => pick(self, BinOp::Le, locals[0], locals[1]),
            "max" => pick(self, BinOp::Ge, locals[0], locals[1]),
            _ => {
                let up = self.new_local(t);
                stmts.push(Stmt::Let(up, pick(self, BinOp::Ge, locals[0], locals[1])));
                pick(self, BinOp::Le, up, locals[2])
            }
        };
        self.at(
            K::Block(Block {
                stmts,
                value: Some(Box::new(value)),
            }),
            t,
            line,
        )
    }

    /// `a.div_floor(b)` and `a.mod(b)` (decision 0051), through `/` and
    /// `%`, so a 0 or an overflow panics as they do: when the remainder
    /// and `b` differ in sign, the quotient goes one down and the
    /// remainder takes `b`.
    fn floor_method(&mut self, name: &str, values: Vec<ir::Expr>, line: u32) -> ir::Expr {
        let t = values[0].ty;
        let mut stmts = Vec::new();
        let mut local = |s: &mut Self, v: ir::Expr| {
            let l = s.new_local(t);
            stmts.push(Stmt::Let(l, v));
            l
        };
        let mut values = values.into_iter();
        let (a, b) = (values.next().expect("a"), values.next().expect("b"));
        let (a, b) = (local(self, a), local(self, b));
        let get = |s: &Self, l: ir::Local| s.local_expr(l, line);
        let bin = |s: &Self, op: BinOp, x: ir::Expr, y: ir::Expr, ty: Type| {
            s.at(K::Binary(op, Box::new(x), Box::new(y)), ty, line)
        };
        let q = bin(self, BinOp::Div, get(self, a), get(self, b), t);
        let r = bin(self, BinOp::Rem, get(self, a), get(self, b), t);
        let (q, r) = (local(self, q), local(self, r));
        let sign = |s: &Self, op: BinOp, l: ir::Local| {
            bin(s, op, get(s, l), s.zero_of(t, line), Type::Bool)
        };
        let both = |s: &Self, x: ir::Expr, y: ir::Expr| {
            s.at(K::And(Box::new(x), Box::new(y)), Type::Bool, line)
        };
        let r_neg_b_pos = both(self, sign(self, BinOp::Lt, r), sign(self, BinOp::Gt, b));
        let r_pos_b_neg = both(self, sign(self, BinOp::Gt, r), sign(self, BinOp::Lt, b));
        let differ = self.at(
            K::Or(Box::new(r_neg_b_pos), Box::new(r_pos_b_neg)),
            Type::Bool,
            line,
        );
        let (adjusted, plain) = if name == "div_floor" {
            let one = self.at(K::Int(1), t, line);
            (bin(self, BinOp::Sub, get(self, q), one, t), get(self, q))
        } else {
            (
                bin(self, BinOp::Add, get(self, r), get(self, b), t),
                get(self, r),
            )
        };
        let value = self.at(
            K::If(Box::new(differ), wrap_value(adjusted), wrap_value(plain)),
            t,
            line,
        );
        self.at(
            K::Block(Block {
                stmts,
                value: Some(Box::new(value)),
            }),
            t,
            line,
        )
    }

    fn zero_of(&self, t: Type, line: u32) -> ir::Expr {
        let kind = match (t, self.shapes.shape(t)) {
            (Type::F64 | Type::F32, _) => K::Float(0.0),
            (_, Some(Shape::Decimal)) => K::Decimal("0".into()),
            _ => K::Int(0),
        };
        self.at(kind, t, line)
    }

    /// `items.sort_by(key)`: the keys from an inlined loop, then a stable
    /// sort of the items by them.
    pub(crate) fn sort_by(
        &mut self,
        list: ir::Expr,
        f: &'a ast::Expr,
        ty: Type,
        span: ast::Span,
    ) -> Lowered<ir::Expr> {
        let line = self.line(span);
        let key = match self.c.expr_types.get(&(f.span.start, f.span.end)) {
            Some(Ty::Fn(_, k)) => self.kir_type(&k.clone(), span)?,
            _ => return Err(self.unsupported("`sort_by` here", span)),
        };
        let keys_ty = self.list_type(key);
        let tmp = self.new_local(list.ty);
        let src = self.local_expr(tmp, line);
        let keys = self.list_loop("map", src, f, keys_ty, span)?;
        let sorted = self.at(
            K::Rt(RtOp::ListSortBy, vec![self.local_expr(tmp, line), keys]),
            ty,
            line,
        );
        Ok(self.at(
            K::Block(Block {
                stmts: vec![Stmt::Let(tmp, list)],
                value: Some(Box::new(sorted)),
            }),
            ty,
            line,
        ))
    }
}

fn wrap_value(e: ir::Expr) -> Block {
    Block {
        stmts: Vec::new(),
        value: Some(Box::new(e)),
    }
}
