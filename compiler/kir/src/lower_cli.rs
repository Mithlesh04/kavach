//! `cli.parse<T>()` in native code (spec 14.6). The runtime reads the
//! arguments and converts each to its field's type (`CliParse`); the
//! compiled code then fills in defaults, which are Kavach expressions,
//! and builds the record, or the variant the command names. The usage
//! text is made here, the same way the interpreter makes it
//! (`compiler/interp/src/cli.rs`).

use crate::ir::{self, BinOp, Block, ExprKind as K, FieldShape, Local, Shape, Stmt, Type, UnOp};
use crate::lower::{Lowered, Lowerer};
use crate::lower_errors::Outcome;
use crate::rt_op::RtOp;
use kavach_ast::{self as ast, ExprKind, ItemKind, TypeBody, TypeKind};

fn option(name: &str) -> String {
    name.replace('_', "-")
}

fn command(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('-');
        }
        out.extend(c.to_lowercase());
    }
    out
}

fn is_flag(t: &ast::TypeExpr) -> bool {
    matches!(&t.kind, TypeKind::Named { path, .. } if path.segments[0].name == "Bool")
}

impl<'a> Lowerer<'a> {
    /// The type declaration of `cli.parse<T>()` if `e` is that call.
    fn cli_parse_target(&self, e: &'a ast::Expr) -> Option<&'a ast::TypeDecl> {
        let ExprKind::Call {
            callee, generics, ..
        } = &e.kind
        else {
            return None;
        };
        let ExprKind::Field { base, name, .. } = &callee.kind else {
            return None;
        };
        let ExprKind::Name(id) = &base.kind else {
            return None;
        };
        let def = self.c.resolution.uses.get(&id.span.start)?;
        if self.std_modules.get(def).map(String::as_str) != Some("cli") || name.name != "parse" {
            return None;
        }
        let TypeKind::Named { path, .. } = &generics.first()?.kind else {
            return None;
        };
        let target = self.c.resolution.uses.get(&path.segments[0].span.start)?;
        let span = self.c.resolution.defs[*target].span?;
        self.c.file.items.iter().find_map(|item| match &item.kind {
            ItemKind::Type(t) if t.name.span == span => Some(t),
            _ => None,
        })
    }

    fn program(&self) -> String {
        std::path::Path::new(&self.c.path)
            .file_stem()
            .map_or("program".into(), |s| s.to_string_lossy().to_string())
    }

    fn option_lines(&self, fields: &[ast::Field], indent: usize) -> Vec<String> {
        let width = 26 - indent;
        fields
            .iter()
            .map(|f| {
                let name = format!("--{}", option(&f.name.name));
                let shown = if is_flag(&f.ty) {
                    name
                } else {
                    format!("{name} <{}>", self.text(f.ty.span).trim_end_matches('?'))
                };
                let note = match (&f.default, &f.ty.kind) {
                    (Some(d), _) if !is_flag(&f.ty) => format!("(default {})", self.text(d.span)),
                    (None, TypeKind::Optional(_)) => "(optional)".into(),
                    (None, _) if !is_flag(&f.ty) => "(required)".into(),
                    _ => String::new(),
                };
                format!("{:indent$}{shown:<width$} {note}", "")
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    fn usage(&self, cmd: Option<&str>, fields: &[ast::Field]) -> String {
        let words = match cmd {
            Some(cmd) => format!("{} {cmd}", self.program()),
            None => self.program(),
        };
        let mut lines = vec![format!("usage: {words} [options]")];
        lines.extend(self.option_lines(fields, 2));
        lines.join("\n")
    }

    fn commands_usage(&self, variants: &[ast::Variant]) -> String {
        let mut lines = vec![format!("usage: {} <command> [options]", self.program())];
        for v in variants {
            lines.push(format!("  {}", command(&v.name.name)));
            lines.extend(self.option_lines(&v.fields, 4));
        }
        lines.join("\n")
    }

    /// `cli.parse<T>()` as an outcome: its value is the record or variant.
    pub(crate) fn cli_parse_outcome(
        &mut self,
        e: &'a ast::Expr,
        err: Type,
    ) -> Option<Lowered<Outcome>> {
        let decl = self.cli_parse_target(e)?;
        Some(self.cli_parse(e, decl, err))
    }

    /// What the runtime gets for a group of fields: per field
    /// `option|required|kind|type text`; and the optional of each field's
    /// plain type, which the runtime fills.
    fn cli_specs(
        &mut self,
        fields: &[ast::Field],
        shapes: &[FieldShape],
    ) -> (Vec<String>, Vec<Type>) {
        let mut spec = Vec::new();
        let mut slots = Vec::new();
        for (f, s) in fields.iter().zip(shapes) {
            let plain = match self.shapes.shape(s.ty) {
                Some(Shape::Optional(inner)) => *inner,
                _ => s.ty,
            };
            let required = f.default.is_none()
                && !matches!(f.ty.kind, TypeKind::Optional(_))
                && !is_flag(&f.ty);
            let kind = if is_flag(&f.ty) { "flag" } else { "value" };
            spec.push(format!(
                "{}|{}|{kind}|{}",
                option(&f.name.name),
                required as u8,
                self.text(f.ty.span).trim_end_matches('?')
            ));
            slots.push(self.optional_type(plain));
        }
        (spec, slots)
    }

    /// The fields' values from the runtime's slots, which start at slot
    /// `first` of tuple `given`, with defaults filled in.
    fn cli_values(
        &mut self,
        fields: &'a [ast::Field],
        shapes: &[FieldShape],
        slots: &[Type],
        (given, first): (Local, usize),
        line: u32,
    ) -> Lowered<Vec<ir::Expr>> {
        let mut values = Vec::new();
        for (i, ((f, s), &slot)) in fields.iter().zip(shapes).zip(slots).enumerate() {
            let got = ir::Expr {
                kind: K::Index(
                    Box::new(self.local_expr(given, line)),
                    Box::new(int(first + i, line)),
                ),
                ty: slot,
                line,
            };
            let plain = match self.shapes.shape(slot) {
                Some(Shape::Optional(inner)) => *inner,
                _ => slot,
            };
            values.push(match &f.default {
                // No default: an optional field takes what was given; a
                // required one was checked by the runtime.
                None if s.ty == slot => got,
                None => ir::Expr {
                    kind: K::Unwrap(Box::new(got)),
                    ty: plain,
                    line,
                },
                Some(d) => {
                    let tmp = self.new_local(slot);
                    let present = ir::Expr {
                        kind: K::IsSome(Box::new(self.local_expr(tmp, line))),
                        ty: Type::Bool,
                        line,
                    };
                    let inside = ir::Expr {
                        kind: K::Unwrap(Box::new(self.local_expr(tmp, line))),
                        ty: plain,
                        line,
                    };
                    let inside = self.coerce(inside, s.ty)?;
                    let fallback = self.expr(d)?;
                    let fallback = self.coerce(fallback, s.ty)?;
                    let pick = ir::Expr {
                        kind: K::If(Box::new(present), wrap(inside), wrap(fallback)),
                        ty: s.ty,
                        line,
                    };
                    ir::Expr {
                        kind: K::Block(Block {
                            stmts: vec![Stmt::Let(tmp, got)],
                            value: Some(Box::new(pick)),
                        }),
                        ty: s.ty,
                        line,
                    }
                }
            });
        }
        Ok(values)
    }

    fn cli_parse(
        &mut self,
        e: &'a ast::Expr,
        decl: &'a ast::TypeDecl,
        err: Type,
    ) -> Lowered<Outcome> {
        let line = self.line(e.span);
        let target = self.ty_of(e)?;
        // The groups of options, with their fields' shapes: the record's,
        // or one per command of a union.
        let commands = match &decl.body {
            TypeBody::Union(vs) => Some(&vs[..]),
            TypeBody::Record(_) | TypeBody::Alias(_) => None,
        };
        let groups: Vec<(&'a [ast::Field], Vec<FieldShape>)> =
            match (&decl.body, self.shapes.shape(target).cloned()) {
                (TypeBody::Record(r), Some(Shape::Record { fields, .. })) => {
                    vec![(&r.fields[..], fields)]
                }
                (TypeBody::Union(vs), Some(Shape::Union { variants, .. })) => vs
                    .iter()
                    .zip(variants)
                    .map(|(v, s)| (&v.fields[..], s.fields))
                    .collect(),
                _ => return Err(self.unsupported("`cli.parse` here", e.span)),
            };
        // For a union, the spec has a line `@command` before each group's
        // lines, the usage texts are the whole usage then each command's
        // (split by `\0`), and the tuple starts with the command's index.
        let mut spec = Vec::new();
        let mut usage = vec![match commands {
            Some(vs) => self.commands_usage(vs),
            None => self.usage(None, groups[0].0),
        }];
        let mut slots = if commands.is_some() {
            vec![Type::I64]
        } else {
            Vec::new()
        };
        let mut firsts = Vec::new();
        for (n, (fields, shapes)) in groups.iter().enumerate() {
            if let Some(vs) = commands {
                let cmd = command(&vs[n].name.name);
                spec.push(format!("@{cmd}"));
                usage.push(self.usage(Some(&cmd), fields));
            }
            let (lines, types) = self.cli_specs(fields, shapes);
            spec.extend(lines);
            firsts.push((slots.len(), types.clone()));
            slots.extend(types);
        }
        let str_ty = self.str_type();
        let text = |s: String| ir::Expr {
            kind: K::Str(s),
            ty: str_ty,
            line,
        };
        let tuple = self.tuple_type(slots);
        let call = ir::Expr {
            kind: K::Rt(
                RtOp::CliParse,
                vec![text(spec.join("\n")), text(usage.join("\0"))],
            ),
            ty: tuple,
            line,
        };
        let (given, error, value) = (
            self.new_local(tuple),
            self.new_local(err),
            self.new_local(target),
        );
        // The record, or a chain of `if`s on the command's index.
        let mut built: Option<ir::Expr> = None;
        for (n, ((fields, shapes), (first, types))) in groups.iter().zip(&firsts).enumerate().rev()
        {
            let values = self.cli_values(fields, shapes, types, (given, *first), line)?;
            let this = ir::Expr {
                kind: K::Record {
                    variant: n as u32,
                    fields: values,
                },
                ty: target,
                line,
            };
            built = Some(match built {
                None => this,
                Some(rest) => {
                    let index = ir::Expr {
                        kind: K::Index(
                            Box::new(self.local_expr(given, line)),
                            Box::new(int(0, line)),
                        ),
                        ty: Type::I64,
                        line,
                    };
                    let is = ir::Expr {
                        kind: K::Binary(BinOp::Eq, Box::new(index), Box::new(int(n, line))),
                        ty: Type::Bool,
                        line,
                    };
                    ir::Expr {
                        kind: K::If(Box::new(is), wrap(this), wrap(rest)),
                        ty: target,
                        line,
                    }
                }
            });
        }
        let built = built.expect("a record or union has a group of options");
        let built = match commands {
            Some(_) => built,
            None => self.with_invariants(built)?,
        };
        let ok = ir::Expr {
            kind: K::Unary(UnOp::Not, Box::new(self.failed(error, line))),
            ty: Type::Bool,
            line,
        };
        let store = ir::Expr {
            kind: K::Assign(value, Box::new(built)),
            ty: Type::Unit,
            line,
        };
        let fill = ir::Expr {
            kind: K::If(
                Box::new(ok),
                Block {
                    stmts: vec![Stmt::Expr(store)],
                    value: None,
                },
                Block::default(),
            ),
            ty: Type::Unit,
            line,
        };
        let all = Block {
            stmts: vec![
                Stmt::LetResult {
                    value: given,
                    error,
                    call,
                },
                Stmt::Expr(fill),
            ],
            value: None,
        };
        Ok(Outcome {
            plain: None,
            stmt: Stmt::Expr(ir::Expr {
                kind: K::Block(all),
                ty: Type::Unit,
                line,
            }),
            value,
            error,
            ok: target,
            err,
        })
    }
}

fn int(n: usize, line: u32) -> ir::Expr {
    ir::Expr {
        kind: K::Int(n as i128),
        ty: Type::I64,
        line,
    }
}

fn wrap(e: ir::Expr) -> Block {
    Block {
        stmts: Vec::new(),
        value: Some(Box::new(e)),
    }
}
