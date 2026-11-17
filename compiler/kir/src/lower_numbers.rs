//! Lowering methods of numbers (spec 14.2): `abs`, `min`, `max`, `clamp`,
//! `div_floor` and `mod`, and the zero of a number type.

use crate::ir::{self, BinOp, Block, ExprKind as K, Shape, Stmt, Type, UnOp};
use crate::lower::Lowerer;
use crate::lower_std::wrap_value;
use crate::rt_op::RtOp;

impl Lowerer<'_> {
    /// `abs`, `min`, `max` and `clamp`, written with compares, so they are
    /// as fast as hand-written code.
    pub(crate) fn number_method(
        &mut self,
        name: &str,
        values: Vec<ir::Expr>,
        line: u32,
    ) -> ir::Expr {
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
        // `a` when `a op b`, else `b`; for floats, `b` only when `b` is
        // strictly on its side, else `a + b`: `NaN` when either is (14.2).
        let float = t.is_float();
        let pick = |s: &Self, op: BinOp, a: ir::Local, b: ir::Local| {
            let test = |op, x, y| {
                s.at(
                    K::Binary(op, Box::new(get(s, x)), Box::new(get(s, y))),
                    Type::Bool,
                    line,
                )
            };
            let otherwise = if float {
                let strict = if op == BinOp::Le {
                    BinOp::Lt
                } else {
                    BinOp::Gt
                };
                let nan = s.at(
                    K::Binary(BinOp::Add, Box::new(get(s, a)), Box::new(get(s, b))),
                    t,
                    line,
                );
                s.at(
                    K::If(
                        Box::new(test(strict, b, a)),
                        wrap_value(get(s, b)),
                        wrap_value(nan),
                    ),
                    t,
                    line,
                )
            } else {
                get(s, b)
            };
            s.at(
                K::If(
                    Box::new(test(op, a, b)),
                    wrap_value(get(s, a)),
                    wrap_value(otherwise),
                ),
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
                // `lo > hi` breaks `clamp`'s contract (14.2).
                let reversed = self.at(
                    K::Binary(
                        BinOp::Gt,
                        Box::new(get(self, locals[1])),
                        Box::new(get(self, locals[2])),
                    ),
                    Type::Bool,
                    line,
                );
                let text = self.str_type();
                let why = self.at(
                    K::Str("contract failed: `clamp` needs `lo <= hi`".into()),
                    text,
                    line,
                );
                let stop = Block {
                    stmts: vec![Stmt::Expr(self.at(
                        K::Rt(RtOp::PanicText, vec![why]),
                        Type::Unit,
                        line,
                    ))],
                    value: None,
                };
                stmts.push(Stmt::Expr(self.at(
                    K::If(Box::new(reversed), stop, Block::default()),
                    Type::Unit,
                    line,
                )));
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
    pub(crate) fn floor_method(
        &mut self,
        name: &str,
        values: Vec<ir::Expr>,
        line: u32,
    ) -> ir::Expr {
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
        // `mod` needs no quotient: `Int.MIN.mod(-1)` is 0, as `%`.
        let q = match name {
            "div_floor" => bin(self, BinOp::Div, get(self, a), get(self, b), t),
            _ => self.at(K::Int(0), t, line),
        };
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

    pub(crate) fn zero_of(&self, t: Type, line: u32) -> ir::Expr {
        let kind = match (t, self.shapes.shape(t)) {
            (Type::F64 | Type::F32, _) => K::Float(0.0),
            // `kv_dec_new` makes a `BigInt` too, by the type.
            (_, Some(Shape::Decimal | Shape::BigInt)) => K::Decimal("0".into()),
            _ => K::Int(0),
        };
        self.at(kind, t, line)
    }
}
