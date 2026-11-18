//! `std.reflect` (spec 14.32): `T.members()`, `m.members()` and
//! `x.has_member(name)`, worked out while compiling. A list is written as
//! JSON, which `std.reflect` reads back (`members_from`); a `has_member`
//! is a `Bool` known here. Both are kept by the call's span for the
//! interpreter and native code.

use crate::checker::{Checker, FnSig, Shape};
use crate::ty::{Prim, Ty};
use kavach_ast::{Arg, Expr, ExprKind, Ident, Span, StrPart};
use kavach_resolver::{DefId, DefKind};

/// `text` as a JSON string.
fn quoted(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// One member, as `std.reflect`'s `Member` reads it.
struct Row {
    name: String,
    kind: &'static str,
    params: Vec<(String, String)>,
    result: String,
    /// `None` for a built-in member; `Some("")` for one declared in the
    /// type or module; the module's name for one an `extend` added.
    origin: Option<String>,
}

impl Row {
    fn json(&self) -> String {
        let params: Vec<String> = (self.params.iter())
            .map(|(n, t)| format!("{{\"name\":{},\"type_name\":{}}}", quoted(n), quoted(t)))
            .collect();
        let origin = match &self.origin {
            None => "\"BuiltIn\"".to_string(),
            Some(m) if m.is_empty() => "\"Declared\"".to_string(),
            Some(m) => format!("{{\"Extended\":{{\"module\":{}}}}}", quoted(m)),
        };
        format!(
            "{{\"name\":{},\"kind\":\"{}\",\"params\":[{}],\"result\":{},\"origin\":{origin}}}",
            quoted(&self.name),
            self.kind,
            params.join(","),
            quoted(&self.result)
        )
    }
}

fn json(rows: &[Row]) -> String {
    let rows: Vec<String> = rows.iter().map(Row::json).collect();
    format!("[{}]", rows.join(","))
}

impl Checker<'_> {
    /// `T.members()` and `m.members()`: `None` for any other call.
    pub(crate) fn reflect_static(
        &mut self,
        def: DefId,
        name: &Ident,
        args: &[Arg],
        span: Span,
    ) -> Option<Ty> {
        if !args.is_empty() || name.name != "members" {
            return None;
        }
        let rows = match self.res.defs[def].kind {
            DefKind::Type | DefKind::Builtin => {
                let ty = self.reflected_type(def)?;
                self.type_rows(&ty)
            }
            DefKind::Import | DefKind::Module => {
                let module = *self.res.module_defs.get(&def)?;
                self.module_rows(module)
            }
            _ => return None,
        };
        let Some(made) = self.res.builtin_type("std.reflect", "Member") else {
            self.error(span, "this needs `std.reflect` (spec 14.32)");
            return Some(Ty::Error);
        };
        self.reflect_texts
            .insert((span.start, span.end), json(&rows));
        Some(Ty::List(Box::new(Ty::Adt(made, Vec::new()))))
    }

    /// `x.has_member("name")`: `None` for any other call.
    pub(crate) fn reflect_has_member(
        &mut self,
        recv: &Ty,
        name: &Ident,
        args: &[Arg],
        call: &Expr,
    ) -> Option<Ty> {
        if name.name != "has_member" {
            return None;
        }
        let asked = match args {
            [a] if a.name.is_none() => match &a.value.kind {
                ExprKind::Str(parts) => match parts.as_slice() {
                    [StrPart::Text(t)] => Some(t.clone()),
                    [] => Some(String::new()),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        };
        let Some(asked) = asked else {
            self.error(
                call.span,
                "`has_member` takes the member's name as a text literal (spec 14.32)",
            );
            return Some(Ty::Prim(Prim::Bool));
        };
        let has = self.type_rows(recv).iter().any(|r| r.name == asked);
        self.reflect_bools
            .insert((call.span.start, call.span.end), has);
        Some(Ty::Prim(Prim::Bool))
    }

    /// The type `T` of `T.members()`: a declared type, or a built-in one
    /// by its name, with `T`, `K` and `V` for a collection's items.
    fn reflected_type(&self, def: DefId) -> Option<Ty> {
        if self.res.defs[def].kind == DefKind::Type {
            return Some(Ty::Adt(def, Vec::new()));
        }
        let item = || Box::new(Ty::Builtin("T", Vec::new()));
        Some(match self.res.defs[def].name.as_str() {
            "List" => Ty::List(item()),
            "Set" => Ty::Set(item()),
            "Dict" => Ty::Map(
                Box::new(Ty::Builtin("K", Vec::new())),
                Box::new(Ty::Builtin("V", Vec::new())),
            ),
            "Ring" => Ty::Builtin("Ring", vec![*item()]),
            other => match Prim::from_name(other) {
                Some(p) => Ty::Prim(p),
                None => Ty::Builtin(crate::ty::builtin_name(other), Vec::new()),
            },
        })
    }

    /// Every field and method of `ty` that code here can use, by name.
    fn type_rows(&mut self, ty: &Ty) -> Vec<Row> {
        let mut rows = Vec::new();
        let ty = self.inf.resolve(ty);
        for name in crate::method_names::NAMES {
            if let Some(m) = self.builtin_method(&ty, name) {
                let names = (b'a'..=b'z').map(|c| (c as char).to_string());
                let mut params: Vec<(String, String)> = names
                    .zip(&m.params)
                    .map(|(n, t)| (n, self.show(t)))
                    .collect();
                params.extend(m.named.iter().map(|(n, t)| (n.to_string(), self.show(t))));
                let result = self.shown_result(&m.ret);
                let name = name.to_string();
                rows.push(Row {
                    name,
                    kind: "Method",
                    params,
                    result,
                    origin: None,
                });
            }
        }
        let here = self.ctx_package();
        if let Ty::Adt(def, _) = &ty
            && let Some(Shape::Record(fields)) = self.types.get(def).map(|t| t.shape.clone())
        {
            let own = self.res.defs[*def]
                .span
                .map(|s| self.res.module_at(s.start));
            let home = own.map(|m| self.res.modules[m].package.clone());
            for f in fields {
                if !f.is_pub && home.as_deref() != here.as_deref() {
                    continue;
                }
                let result = self.show(&f.ty);
                let origin = Some(String::new());
                rows.push(Row {
                    name: f.name,
                    kind: "Field",
                    params: Vec::new(),
                    result,
                    origin,
                });
            }
        }
        let own: Vec<(String, FnSig)> = match &ty {
            Ty::Adt(def, _) => (self.methods.get(def).into_iter().flatten())
                .map(|(n, s)| (n.clone(), s.clone()))
                .collect(),
            other => {
                let kind = match other {
                    Ty::Prim(p) => Some(p.name()),
                    t => crate::methods::collection_kind(t),
                };
                (kind
                    .and_then(|k| self.ext_methods.get(k))
                    .into_iter()
                    .flatten())
                .map(|(n, s)| (n.clone(), s.clone()))
                .collect()
            }
        };
        for (name, sig) in own.into_iter().filter(|(_, s)| s.has_self) {
            let module =
                (sig.def.and_then(|d| self.res.defs[d].span)).map(|s| self.res.module_at(s.start));
            let private = sig.def.is_some_and(|d| self.private_methods.contains(&d));
            let package = module.map(|m| self.res.modules[m].package.clone());
            if private && package.as_deref() != here.as_deref() {
                continue;
            }
            let extended = sig.def.is_some_and(|d| self.extended_methods.contains(&d));
            let origin = match (extended, module) {
                (true, Some(m)) => self.res.modules[m].name.clone(),
                _ => String::new(),
            };
            let params = (sig.params.iter())
                .map(|p| (p.name.clone(), self.show(&p.ty)))
                .collect();
            let result = self.shown_result(&sig.ret);
            let origin = Some(origin);
            rows.push(Row {
                name,
                kind: "Method",
                params,
                result,
                origin,
            });
        }
        rows.sort_by(|a, b| a.name.cmp(&b.name));
        rows
    }

    fn shown_result(&self, t: &Ty) -> String {
        match t {
            Ty::Unit => "Nothing".to_string(),
            t => self.show(t),
        }
    }

    /// The package of the code being checked.
    fn ctx_package(&self) -> Option<String> {
        let at = self.current_fn.and_then(|d| self.res.defs[d].span)?;
        Some(
            self.res.modules[self.res.module_at(at.start)]
                .package
                .clone(),
        )
    }

    /// What module number `module` declares `pub`, by name.
    fn module_rows(&self, module: usize) -> Vec<Row> {
        let mut rows = Vec::new();
        for (d, def) in self.res.defs.iter().enumerate() {
            let Some(span) = def.span else { continue };
            if self.res.module_at(span.start) != module || !self.res.public.contains(&d) {
                continue;
            }
            let (kind, params, result) = match def.kind {
                DefKind::Fn => match self.fns.get(&d) {
                    Some(sig) => {
                        let ps = (sig.params.iter())
                            .map(|p| (p.name.clone(), self.show(&p.ty)))
                            .collect();
                        ("Function", ps, self.shown_result(&sig.ret))
                    }
                    None => continue,
                },
                DefKind::Type | DefKind::Interface | DefKind::Capability => {
                    ("Type", Vec::new(), def.name.clone())
                }
                DefKind::Local { .. } if self.res.module_consts.contains(&d) => {
                    let t = self.locals.get(&d).map_or(String::new(), |t| self.show(t));
                    ("Constant", Vec::new(), t)
                }
                _ => continue,
            };
            let (name, origin) = (def.name.clone(), Some(String::new()));
            rows.push(Row {
                name,
                kind,
                params,
                result,
                origin,
            });
        }
        rows.sort_by(|a, b| a.name.cmp(&b.name));
        rows
    }
}
