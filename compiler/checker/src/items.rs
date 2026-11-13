//! Declarations, function bodies, blocks and statements.

use crate::checker::{Checker, Ctx, FieldInfo, Shape};
use crate::ty::{Prim, Ty};
use kavach_ast::*;

const BOOL: Ty = Ty::Prim(Prim::Bool);

impl Checker<'_> {
    pub(crate) fn check_file(&mut self, file: &File) {
        let mut top_level = Vec::new();
        for item in &file.items {
            match &item.kind {
                ItemKind::Fn(f) => self.function(f, None),
                ItemKind::Literal(l) => self.check_literal(l),
                ItemKind::Type(t) => self.type_decl(t),
                ItemKind::Extend(e) => {
                    let target = self.lower(&e.target, None);
                    self.extending = match &target {
                        Ty::Adt(def, _) => Some(*def),
                        _ => None,
                    };
                    for m in &e.methods {
                        if let ItemKind::Fn(f) = &m.kind {
                            self.function(f, Some(target.clone()));
                        }
                    }
                    self.extending = None;
                }
                ItemKind::Agent(a) => {
                    self.check_agent_rules(&a.members);
                    for member in &a.members {
                        if let AgentMember::On(f) = member {
                            self.function(f, None);
                        }
                    }
                }
                ItemKind::Test(t) => {
                    // What it may use (spec 11.8): `uses` and `with`.
                    let path = |p: &Path| {
                        let names: Vec<&str> = p.segments.iter().map(|s| s.name.as_str()).collect();
                        names.join(".")
                    };
                    let mut allowed: std::collections::BTreeSet<String> =
                        t.uses.iter().map(path).collect();
                    allowed.extend(t.with.iter().map(|(c, _)| c.name.clone()));
                    self.test_needs.push(crate::capabilities::TestNeeds {
                        allowed,
                        needs: Vec::new(),
                    });
                    self.current_test = Some(self.test_needs.len() - 1);
                    self.with_ctx("test", Ty::Unit, Some(error_ty()), None, |c| {
                        c.block_stmt(&t.body, None)
                    });
                    self.current_test = None;
                }
                ItemKind::Stmt(s) => top_level.push(s.clone()),
                ItemKind::Interface(_) | ItemKind::Capability(_) | ItemKind::Use(_) => {}
            }
        }
        // Top-level statements are the body of `main` (spec 1.3); like
        // `fn main() ! Error`, they may pass errors on with `try`.
        self.with_ctx("main", Ty::Unit, Some(error_ty()), None, |c| {
            for s in &top_level {
                c.stmt(s);
            }
        });
    }

    pub(crate) fn with_ctx(
        &mut self,
        name: &str,
        ret: Ty,
        errors: Option<Ty>,
        self_ty: Option<Ty>,
        f: impl FnOnce(&mut Self),
    ) {
        let outer = self.ctx.replace(Ctx {
            name: name.to_string(),
            ret,
            ret_secret: false,
            errors,
            self_ty,
        });
        f(self);
        self.check_deferred(None);
        self.ctx = outer;
    }

    fn type_decl(&mut self, t: &TypeDecl) {
        let Some(&def) = self.decl.get(&t.name.span.start) else {
            return;
        };
        let generics = self
            .types
            .get(&def)
            .map(|i| i.generics.clone())
            .unwrap_or_default();
        let self_ty = Ty::Adt(def, generics.iter().map(|g| Ty::Param(*g)).collect());
        let shape = self.types.get(&def).map(|i| i.shape.clone());
        if let (TypeBody::Union(variants), Some(Shape::Union(infos))) = (&t.body, &shape) {
            for (v, info) in variants.iter().zip(infos) {
                self.field_defaults(&v.fields, &info.fields);
            }
        }
        let TypeBody::Record(r) = &t.body else { return };
        if let Some(Shape::Record(fields)) = &shape {
            self.field_defaults(&r.fields, fields);
        }
        self.record = Some(def);
        self.in_contract = true;
        for inv in &r.invariants {
            self.check_expect(inv, &BOOL);
        }
        self.in_contract = false;
        self.record = None;
        for m in &r.methods {
            if let ItemKind::Fn(f) = &m.kind {
                self.function(f, Some(self_ty.clone()));
            }
        }
    }

    /// Each default must have its field's type.
    fn field_defaults(&mut self, fields: &[Field], infos: &[FieldInfo]) {
        for (field, info) in fields.iter().zip(infos) {
            if let Some(default) = &field.default {
                self.check_expect(default, &info.ty);
            }
        }
    }

    pub(crate) fn function(&mut self, f: &FnDecl, self_ty: Option<Ty>) {
        let outer = self.current_fn;
        self.current_fn = self.decl.get(&f.name.span.start).copied();
        let outer_loops = std::mem::take(&mut self.loops);
        self.function_body(f, self_ty);
        self.current_fn = outer;
        self.loops = outer_loops;
    }

    fn function_body(&mut self, f: &FnDecl, self_ty: Option<Ty>) {
        let sig = self.sig_of(f, self_ty.as_ref());
        let params = f.params.iter().filter(|p| p.name.name != "self");
        for (p, info) in params.zip(&sig.params) {
            if let Some(default) = &p.default {
                self.check_expect(default, &info.ty);
            }
            if let Some(&def) = self.decl.get(&p.name.span.start) {
                self.locals.insert(def, info.ty.clone());
                if p.is_secret {
                    self.secret_defs.insert(def);
                }
            }
        }
        let name = f.name.name.clone();
        self.with_ctx(&name, sig.ret.clone(), sig.errors.clone(), self_ty, |c| {
            if let Some(ctx) = c.ctx.as_mut() {
                ctx.ret_secret = f.ret_secret;
            }
            c.in_contract = true;
            for e in f.requires.iter().chain(&f.ensures) {
                c.check_expect(e, &BOOL);
            }
            c.in_example = true;
            for x in &f.examples {
                c.example(x);
            }
            c.in_example = false;
            c.in_contract = false;
            let Some(body) = &f.body else { return };
            if f.ret.is_none() {
                return c.block_stmt(body, Some(&name));
            }
            let actual = c.block_value(body, Some(&sig.ret));
            if !f.ret_secret
                && let Some(StmtKind::Expr(last)) = body.stmts.last().map(|s| &s.kind)
            {
                c.check_secret_return(last, &name);
            }
            if matches!(c.inf.shallow(&actual), Ty::Unit) && !matches!(sig.ret, Ty::Unit) {
                let want = c.show(&sig.ret);
                c.error(
                    f.name.span,
                    format!("`{name}` must return `{want}`, but its last line has no value"),
                );
            }
        });
    }

    fn example(&mut self, x: &Example) {
        let Some(fails) = &x.fails else {
            self.check_expect(&x.expr, &BOOL);
            return;
        };
        match self.infer(&x.expr, None) {
            Ty::Result(_, err) => {
                self.check_expect(fails, &err);
            }
            Ty::Error => {}
            _ => self.error(x.expr.span, "`fails` needs a call that can fail"),
        }
    }

    /// A block whose last expression is its value.
    pub(crate) fn block_value(&mut self, b: &Block, expected: Option<&Ty>) -> Ty {
        let Some((last, init)) = b.stmts.split_last() else {
            return Ty::Unit;
        };
        for s in init {
            self.stmt(s);
        }
        match (&last.kind, expected) {
            (StmtKind::Expr(e), Some(t)) => self.check_expect(e, t),
            (StmtKind::Expr(e), None) => self.value(e, None),
            _ => {
                self.stmt(last);
                Ty::Unit
            }
        }
    }

    /// A block whose value is not used. In a function without `->`
    /// (`no_arrow` names it), a value on the last line is an error (spec 3.1).
    pub(crate) fn block_stmt(&mut self, b: &Block, no_arrow: Option<&str>) {
        for (i, s) in b.stmts.iter().enumerate() {
            if let (Some(name), StmtKind::Expr(e), true) =
                (no_arrow, &s.kind, i + 1 == b.stmts.len())
                && !is_statement_like(e)
            {
                let t = self.value(e, None);
                if !self.is_nothing(&t) && !self.droppable(e) {
                    let ty = self.show(&t);
                    let message = match (&e.kind, name) {
                        (ExprKind::Call { .. }, "main") => format!(
                            "the result of {} is unused: use it, or write `const _ = …` to throw it away",
                            self.describe(e)
                        ),
                        (ExprKind::Call { .. }, _) => format!(
                            "the result of {} is unused: use it, write `const _ = …`, or add `-> {ty}` if `{name}` should return it",
                            self.describe(e)
                        ),
                        _ => format!(
                            "`{name}` has no `->`, so it returns nothing, but its last line is a value: write `-> {ty}`"
                        ),
                    };
                    self.error(e.span, message);
                }
                continue;
            }
            self.stmt(s);
        }
    }

    pub(crate) fn stmt(&mut self, s: &Stmt) {
        match &s.kind {
            StmtKind::Let {
                pattern, ty, value, ..
            } => {
                if ty.is_none() && self.defer_lambda(pattern, value) {
                    return;
                }
                let declared = ty.as_ref().map(|t| self.lower(t, None));
                let actual = match &declared {
                    Some(t) => self.check_expect(value, t),
                    None => self.value(value, None),
                };
                if matches!(self.inf.shallow(&actual), Ty::Unit) {
                    let what = self.describe(value);
                    self.error(
                        value.span,
                        format!("{what} returns nothing, so there is no value to keep"),
                    );
                }
                let ty = declared.unwrap_or(actual);
                if !self.destructure(pattern, &ty, value) {
                    self.bind(pattern, &ty);
                }
                if let PatternKind::Binding(id) = &pattern.kind
                    && self.is_secret(value)
                    && let Some(&def) = self.decl.get(&id.span.start)
                {
                    self.secret_defs.insert(def);
                }
            }
            StmtKind::Assign { target, op, value } => {
                let t = self.value(target, None);
                if let Some(bin) = op.binary() {
                    self.check_compound(bin, target, &t);
                }
                self.check_unsafe_index_target(target);
                self.check_captured_change(target);
                self.check_expect(value, &t);
                self.check_arena_store(target, value);
            }
            StmtKind::For {
                label,
                pattern,
                iter,
                body,
            } => {
                let t = self.value(iter, None);
                let item = self.item_type(&t, iter);
                self.bind(pattern, &item);
                self.loop_body(label, body);
            }
            StmtKind::While { label, cond, body } => {
                self.condition(cond);
                self.loop_body(label, body);
            }
            StmtKind::Assert(e) => {
                self.check_expect(e, &BOOL);
            }
            StmtKind::Expr(e) => self.expr_stmt(e),
        }
    }

    /// `count += 1` (or `log.push(x)`) inside a lambda or `spawn` block,
    /// where `count` is a variable from outside: only the copy changes
    /// (spec 3.3).
    pub(crate) fn check_captured_change(&mut self, target: &Expr) {
        let Some(&(body, what)) = self.copies.last() else {
            return;
        };
        let mut root = target;
        while let ExprKind::Field { base, .. }
        | ExprKind::TupleField { base, .. }
        | ExprKind::Index { base, .. } = &root.kind
        {
            root = base;
        }
        let ExprKind::Name(id) = &root.kind else {
            return;
        };
        let Some(&def) = self.res.uses.get(&id.span.start) else {
            return;
        };
        let d = &self.res.defs[def];
        let outside = matches!(d.kind, kavach_resolver::DefKind::Local { .. })
            && d.span.is_some_and(|s| s.start < body);
        if outside {
            let message = format!(
                "`{}` is a copy inside this {what}: the change doesn't reach the outside. Give the value back instead, or use `Shared` to share it (spec 3.3)",
                id.name
            );
            self.warn(target.span, &message);
        }
    }

    /// A loop's body, inside the loop labeled `label` (spec 4.6).
    fn loop_body(&mut self, label: &Option<Ident>, body: &Block) {
        if let Some(l) = label
            && self.loops.iter().flatten().any(|n| *n == l.name)
        {
            self.error(
                    l.span,
                    format!("a loop inside the loop `{0}` can't be named `{0}` too: give it a name of its own", l.name),
                );
        }
        self.loops.push(label.as_ref().map(|l| l.name.clone()));
        self.block_stmt(body, None);
        self.loops.pop();
    }

    /// `break` or `continue` (`what`), maybe to the loop labeled `label`:
    /// one must be around it, in this function (spec 4.6).
    pub(crate) fn check_jump(&mut self, label: &Option<Ident>, what: &str, span: Span) {
        let Some(l) = label else {
            return;
        };
        if self.loops.iter().flatten().any(|n| *n == l.name) {
            return;
        }
        let named: Vec<String> = self
            .loops
            .iter()
            .flatten()
            .map(|n| format!("`{n}`"))
            .collect();
        let around = match (named.len(), self.copies.is_empty()) {
            (0, false) => "it is inside a lambda or `spawn`, whose body is a function of its own, so it can't leave a loop outside it".to_string(),
            (0, true) => "no loop around it has a label".to_string(),
            _ => format!("the loops around it are named {}", named.join(", ")),
        };
        self.error(
            span,
            format!("`{what} {}` names no loop around it: {around}. A label goes before its loop, as `{}: for … {{`, in the same function (spec 4.6)", l.name, l.name),
        );
    }

    /// `counter.update(n => n + 1)`: the new value `update` gives may be
    /// left unused (spec 6.2).
    fn droppable(&self, e: &Expr) -> bool {
        self.updates.contains(&(e.span.start, e.span.end))
    }

    /// An expression on its own line: its value must not be thrown away (spec 3.7).
    pub(crate) fn expr_stmt(&mut self, e: &Expr) {
        if is_statement_like(e) {
            self.statement_like(e);
            return;
        }
        let t = self.value(e, None);
        if !self.is_nothing(&t) && !self.droppable(e) {
            let message = match &e.kind {
                ExprKind::Call { .. } => format!(
                    "the result of {} is unused: use it, or write `const _ = …` to throw it away",
                    self.describe(e)
                ),
                _ => "this value is unused".to_string(),
            };
            self.error(e.span, message);
        }
    }

    /// The element type when looping over `t`.
    fn item_type(&mut self, t: &Ty, iter: &Expr) -> Ty {
        match self.inf.resolve(t) {
            Ty::List(item) | Ty::Set(item) | Ty::Range(item) => *item,
            // `for tick in time.every_ms(…)` counts rounds (spec 14.4).
            Ty::Builtin("Ticker", _) => Ty::Prim(Prim::Int),
            // Values taken until the channel is closed and empty (spec 9.4).
            Ty::Builtin("Receiver", args) => args.first().cloned().unwrap_or(Ty::Error),
            // A file read as the loop goes (spec 14.3).
            Ty::Builtin("Lines", _) => Ty::Prim(Prim::String),
            Ty::Builtin("Chunks", _) => Ty::Prim(Prim::Bytes),
            Ty::Map(k, v) => Ty::Tuple(vec![*k, *v]),
            Ty::Error | Ty::Var(_) => Ty::Error,
            other => {
                let shown = self.show(&other);
                self.error(
                    iter.span,
                    format!("can't loop over `{shown}`: `for` needs a list, set, map or range"),
                );
                Ty::Error
            }
        }
    }

    /// Conditions must be `Bool`; there is no truthiness (spec 4.1).
    pub(crate) fn condition(&mut self, cond: &Expr) {
        let t = self.value(cond, Some(&BOOL));
        if !self.coerce(&t, &BOOL) {
            let shown = self.show(&t);
            self.error(cond.span, format!("conditions must be `Bool`, not `{shown}`: there is no truthiness, so compare explicitly (for example `!= 0`)"));
        }
    }
}

/// `if`, `match` and blocks used as statements: their branches are
/// statements too, so they have no value to lose.
pub(crate) fn is_statement_like(e: &Expr) -> bool {
    matches!(
        e.kind,
        ExprKind::If { .. }
            | ExprKind::IfLet { .. }
            | ExprKind::Match { .. }
            | ExprKind::Block(_)
            | ExprKind::Arena(_)
            | ExprKind::Unsafe(_)
    )
}

pub(crate) fn error_ty() -> Ty {
    Ty::Builtin("Error", Vec::new())
}
