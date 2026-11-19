// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Which functions only read a parameter (spec 2.20), worked out once
//! every body is checked: a frozen value goes where a `T` is written only
//! to those. And where a sealed value is read as its `T`, for code
//! generation.

use crate::checker::Checker;
use crate::ty::Ty;
use kavach_ast::visit::{VisitMut, walk_expr, walk_stmt};
use kavach_ast::*;
use kavach_resolver::{DefId, DefKind};
use std::collections::{HashMap, HashSet};

pub(crate) type Types = HashMap<(u32, u32), Ty>;
type Memo = HashMap<(DefId, Option<usize>), Result<(), String>>;

impl Checker<'_> {
    /// Every frozen value given where a `T` is written: an error unless
    /// the function only reads that parameter.
    pub(crate) fn check_frozen_passes(&mut self, file: &File) {
        if !self.frozen.used {
            return;
        }
        let types: Types = (self.expr_types.iter())
            .map(|(s, t)| ((s.start, s.end), t.clone()))
            .collect();
        self.frozen
            .reads
            .extend(crate::frozen_sealed::sealed_reads(self, file, &types));
        let passed = std::mem::take(&mut self.frozen.passed);
        if passed.is_empty() {
            return;
        }
        let mut fns = HashMap::new();
        for item in &file.items {
            collect_fns(self, item, &mut fns);
        }
        let mut memo = Memo::new();
        for p in passed {
            let Err(why) = only_reads(self, &fns, &types, &mut memo, p.callee, p.param) else {
                continue;
            };
            let callee = self.res.defs[p.callee].name.clone();
            let fix = match p.sealed {
                true => format!("`{}.unfreeze(key: …)`", p.value),
                false => format!("`{}.unfreeze()`", p.value),
            };
            let message = match p.param {
                None => format!(
                    "`{}` is frozen, so its methods may only read `self`, and `{callee}` {why}: call it on {fix}, a copy that can change (spec 2.20)",
                    p.value
                ),
                Some(_) => format!(
                    "`{}` is frozen, so it goes where a plain type is written only to a function that only reads that parameter, and `{callee}` {why}: pass {fix}, or make the parameter a `Frozen<…>` (spec 2.20)",
                    p.value
                ),
            };
            self.error(p.span, message);
        }
    }
}

fn collect_fns<'f>(c: &Checker, item: &'f Item, out: &mut HashMap<DefId, &'f FnDecl>) {
    let methods: &[Item] = match &item.kind {
        ItemKind::Fn(f) => {
            if let Some(&def) = c.decl.get(&f.name.span.start) {
                out.insert(def, f);
            }
            return;
        }
        ItemKind::Type(t) => match &t.body {
            TypeBody::Record(r) => &r.methods,
            _ => return,
        },
        ItemKind::Extend(e) => &e.methods,
        _ => return,
    };
    for m in methods {
        collect_fns(c, m, out);
    }
}

/// Whether function `def` only reads parameter `param` (`None` for
/// `self`); `Err` says what it does instead. A call back into a function
/// being worked out counts as reading.
fn only_reads(
    c: &Checker,
    fns: &HashMap<DefId, &FnDecl>,
    types: &Types,
    memo: &mut Memo,
    def: DefId,
    param: Option<usize>,
) -> Result<(), String> {
    if let Some(known) = memo.get(&(def, param)) {
        return known.clone();
    }
    memo.insert((def, param), Ok(()));
    let result = (|| {
        let f = fns.get(&def).ok_or("is not known here")?;
        let body = f.body.as_ref().ok_or("has no body here")?;
        let target = match param {
            None => Target::SelfValue,
            Some(i) => {
                let p = (f.params.iter().filter(|p| p.name.name != "self"))
                    .nth(i)
                    .ok_or("has no such parameter")?;
                if p.is_inout {
                    return Err(format!("changes `{}`: it is `inout`", p.name.name));
                }
                Target::Param(
                    *c.decl
                        .get(&p.name.span.start)
                        .ok_or("has no such parameter")?,
                )
            }
        };
        let name = match param {
            Some(i) => (f.params.iter().filter(|p| p.name.name != "self"))
                .nth(i)
                .map_or("it".to_string(), |p| p.name.name.clone()),
            None => "self".to_string(),
        };
        let mut uses = Uses {
            c,
            types,
            target,
            name: name.clone(),
            all: Vec::new(),
            ok: HashSet::new(),
            why: HashMap::new(),
            calls: Vec::new(),
            callees: HashSet::new(),
        };
        uses.expr(&mut Expr {
            kind: ExprKind::Block(body.clone()),
            span: body.span,
        });
        let passed: HashSet<u32> = uses.calls.iter().map(|(s, ..)| s.start).collect();
        if let Some(bad) =
            (uses.all.iter()).find(|s| !uses.ok.contains(&s.start) && !passed.contains(&s.start))
        {
            return Err(uses
                .why
                .get(&bad.start)
                .cloned()
                .unwrap_or_else(|| format!("gives `{name}` back")));
        }
        for (_, callee, p) in std::mem::take(&mut uses.calls) {
            if let Err(why) = only_reads(c, fns, types, memo, callee, p) {
                let g = &c.res.defs[callee].name;
                return Err(format!("passes `{name}` to `{g}`, which {why}"));
            }
        }
        Ok(())
    })();
    memo.insert((def, param), result.clone());
    result
}

enum Target {
    Param(DefId),
    SelfValue,
}

/// The uses of one parameter in a body, and which of them only read it.
struct Uses<'c, 'a> {
    c: &'c Checker<'a>,
    types: &'c Types,
    target: Target,
    name: String,
    all: Vec<Span>,
    ok: HashSet<u32>,
    why: HashMap<u32, String>,
    /// Uses passed on to another function's parameter, or as `self`.
    calls: Vec<(Span, DefId, Option<usize>)>,
    /// The `x.m` of each method call: the call decides about its `x`.
    callees: HashSet<(u32, u32)>,
}

impl Uses<'_, '_> {
    fn is(&self, e: &Expr) -> bool {
        match (&e.kind, &self.target) {
            (ExprKind::SelfValue, Target::SelfValue) => true,
            (ExprKind::Name(id), Target::Param(d)) => {
                self.c.res.uses.get(&id.span.start) == Some(d)
            }
            _ => false,
        }
    }

    fn read(&mut self, e: &Expr) {
        if self.is(e) {
            self.ok.insert(e.span.start);
        }
    }

    fn not(&mut self, e: &Expr, why: &str) {
        if self.is(e) {
            let why = why.replace("{}", &self.name);
            self.why.insert(e.span.start, why);
        }
    }

    /// The method `name` of the type of `recv`, when the program declares it.
    fn method(&self, recv: &Expr, name: &str) -> Option<(DefId, Vec<String>)> {
        let t = self.types.get(&(recv.span.start, recv.span.end))?;
        let Ty::Adt(def, _) = self.c.inf.resolve(t) else {
            return None;
        };
        let sig = self.c.methods.get(&def)?.get(name)?;
        Some((
            sig.def?,
            sig.params.iter().map(|p| p.name.clone()).collect(),
        ))
    }

    /// The arguments of a call to `def`, whose parameters are `params`.
    fn args(&mut self, args: &[Arg], def: DefId, params: &[String]) {
        let mut next = 0;
        for a in args {
            let index = match &a.name {
                Some(n) => params.iter().position(|p| *p == n.name),
                None => {
                    next += 1;
                    Some(next - 1)
                }
            };
            if self.is(&a.value)
                && let Some(i) = index
            {
                self.calls.push((a.value.span, def, Some(i)));
            }
        }
    }

    fn call(&mut self, callee: &Expr, args: &[Arg]) {
        match &callee.kind {
            ExprKind::Field { base, name, .. } => {
                let named = match &base.kind {
                    ExprKind::Name(id) => self
                        .c
                        .res
                        .uses
                        .get(&id.span.start)
                        .map(|d| self.c.res.defs[*d].kind),
                    _ => None,
                };
                if matches!(named, Some(DefKind::Module | DefKind::Type)) {
                    // `orders.total(x)`, `Order.make(x)`: a function of the
                    // program, or of the standard library, which reads.
                    match self.c.res.uses.get(&name.span.start) {
                        Some(&def) if self.c.res.defs[def].kind == DefKind::Fn => {
                            let params: Vec<String> = (self.c.fns.get(&def))
                                .map(|s| s.params.iter().map(|p| p.name.clone()).collect())
                                .unwrap_or_default();
                            self.args(args, def, &params);
                        }
                        Some(_) => args
                            .iter()
                            .for_each(|a| self.not(&a.value, "puts `{}` in a new value")),
                        None => args.iter().for_each(|a| self.read(&a.value)),
                    }
                    return;
                }
                match self.method(base, &name.name) {
                    Some((def, params)) => {
                        if self.is(base) {
                            self.calls.push((base.span, def, None));
                        }
                        self.args(args, def, &params);
                    }
                    None => {
                        if matches!(name.name.as_str(), "freeze" | "unfreeze") {
                            self.not(base, "makes a copy of `{}` with `freeze`");
                        } else {
                            self.read(base);
                        }
                        let changes = (self.types.get(&(base.span.start, base.span.end)))
                            .is_some_and(|t| {
                                crate::methods::changes_in_place(&self.c.inf.resolve(t), &name.name)
                            });
                        for a in args {
                            match changes {
                                true => self.not(&a.value, "puts `{}` into a list"),
                                false => self.read(&a.value),
                            }
                        }
                    }
                }
            }
            ExprKind::Name(id) => match self.c.res.uses.get(&id.span.start) {
                Some(&def) if self.c.res.defs[def].kind == DefKind::Fn => {
                    let params: Vec<String> = (self.c.fns.get(&def))
                        .map(|s| s.params.iter().map(|p| p.name.clone()).collect())
                        .unwrap_or_default();
                    self.args(args, def, &params);
                }
                Some(&def) if self.c.res.defs[def].kind == DefKind::Builtin => {
                    let keeps = matches!(id.name.as_str(), "Shared" | "Weak");
                    for a in args {
                        match keeps {
                            true => self.not(&a.value, "puts `{}` in a `Shared`"),
                            false => self.read(&a.value),
                        }
                    }
                }
                _ => {
                    for a in args {
                        self.not(&a.value, "puts `{}` in a new value");
                    }
                }
            },
            _ => self.read(callee),
        }
    }
}

impl VisitMut for Uses<'_, '_> {
    fn expr(&mut self, e: &mut Expr) {
        if self.is(e) {
            self.all.push(e.span);
        }
        match &e.kind {
            ExprKind::Field { .. } if self.callees.contains(&(e.span.start, e.span.end)) => {}
            ExprKind::Field { base, .. }
            | ExprKind::TupleField { base, .. }
            | ExprKind::With { base, .. } => self.read(base),
            ExprKind::Index { base, index } => {
                self.read(base);
                self.read(index);
            }
            ExprKind::Unary(_, x) => self.read(x),
            ExprKind::Binary(BinaryOp::Default, l, r) => {
                self.not(l, "gives `{}` back");
                self.not(r, "gives `{}` back");
            }
            ExprKind::Binary(_, l, r) => {
                let user = self.c.user_ops.get(&(l.span.start, r.span.end)).copied();
                match user.and_then(|m| self.method(l, m)) {
                    Some((def, _)) => {
                        if self.is(l) {
                            self.calls.push((l.span, def, None));
                        }
                        if self.is(r) {
                            self.calls.push((r.span, def, Some(0)));
                        }
                    }
                    None => {
                        self.read(l);
                        self.read(r);
                    }
                }
            }
            ExprKind::Compare(first, links) => {
                self.read(first);
                for (_, x) in links {
                    self.read(x);
                }
            }
            ExprKind::Str(parts) => {
                for p in parts {
                    if let StrPart::Expr(x) = p {
                        self.read(x);
                    }
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                if arms
                    .iter()
                    .any(|a| matches!(a.pattern.kind, PatternKind::Binding(_)))
                {
                    self.not(scrutinee, "gives `{}` a name of its own in a `match`");
                } else {
                    self.read(scrutinee);
                }
            }
            ExprKind::Call { callee, args, .. } => {
                self.callees.insert((callee.span.start, callee.span.end));
                let (callee, args) = (callee.clone(), args.clone());
                self.call(&callee, &args);
            }
            ExprKind::List(xs) | ExprKind::Tuple(xs) => {
                for x in xs {
                    self.not(x, "puts `{}` in a new value");
                }
            }
            ExprKind::Lambda { body, .. } => self.not(body, "gives `{}` back from a lambda"),
            _ => {}
        }
        walk_expr(self, e);
    }

    fn stmt(&mut self, s: &mut Stmt) {
        match &s.kind {
            StmtKind::Let { pattern, value, .. } => match pattern.kind {
                PatternKind::Binding(_) => self.not(value, "puts `{}` in a variable"),
                _ => self.read(value),
            },
            StmtKind::Assign { value, .. } => self.not(value, "puts `{}` in a variable"),
            StmtKind::For { iter, .. } => self.read(iter),
            _ => {}
        }
        walk_stmt(self, s);
    }
}
