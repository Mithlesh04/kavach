//! Lowering a checked file to KIR: functions, methods, contracts and statements.

use crate::Unsupported;
use crate::collect::collect_functions;
use crate::ir::{self, FuncId, Local, Program, Shape, Stmt, Type};
use crate::types::{Shapes, prim_type};
use kavach_ast::{self as ast, ExprKind, ItemKind, StmtKind, TypeKind};
use kavach_driver::Compiled;
use kavach_resolver::{DefId, DefKind};
use std::collections::HashMap;

pub(crate) type Lowered<T> = Result<T, Unsupported>;

/// Who a method belongs to: a record or union, or a built-in type extended
/// with `extend` (by name).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Owner {
    Adt(DefId),
    Builtin(String),
}

pub(crate) struct Lowerer<'a> {
    pub(crate) c: &'a Compiled,
    pub(crate) decl: HashMap<u32, DefId>,
    pub(crate) fn_ids: HashMap<DefId, FuncId>,
    /// Each `spawn process` call's entry in a child (`lower_process.rs`).
    pub(crate) process_entries: Vec<FuncId>,
    pub(crate) method_ids: HashMap<(Owner, String), FuncId>,
    /// One shim per method a concrete type lends an interface (2.15).
    pub(crate) iface_shims: HashMap<FuncId, FuncId>,
    /// Every function by id: a declaration (with its owner, for methods),
    /// or `None` for one made while lowering (a lambda), kept in `ready`.
    pub(crate) fns: Vec<(Option<&'a ast::FnDecl>, Option<Owner>)>,
    pub(crate) ready: HashMap<FuncId, ir::Function>,
    /// The function-value wrapper of each named function (`lower_closure.rs`).
    pub(crate) wrappers: HashMap<FuncId, FuncId>,
    /// The closure function that makes a value of each function type that
    /// can't fail into one of a type that can (`lower_fn_errors.rs`).
    pub(crate) adapters: HashMap<(Type, Type), FuncId>,
    /// The error of a built-in loop just lowered, given a function that
    /// can fail (`lower_fn_errors.rs`).
    pub(crate) pending_error: Option<Local>,
    pub(crate) shapes: Shapes,
    /// Locals of the function being lowered.
    pub(crate) locals: Vec<Type>,
    pub(crate) local_ids: HashMap<DefId, Local>,
    /// The return type of the function being lowered.
    pub(crate) ret: Type,
    /// Holds the body's value while `ensure` is checked (spec 7.2).
    pub(crate) result: Option<Local>,
    /// The `ensure` clauses of the function being lowered, checked on
    /// every way out of it.
    pub(crate) ensures: &'a [ast::Expr],
    pub(crate) self_local: Option<Local>,
    /// The local holding the record whose `invariant` is being lowered.
    pub(crate) invariant_of: Option<(Local, Type)>,
    /// The value of each enclosing dot lambda (`.price`), innermost last.
    pub(crate) its: Vec<Local>,
    /// Generic functions, compiled once per set of type arguments
    /// (`lower_generic.rs`).
    pub(crate) generic_fns: HashMap<DefId, &'a ast::FnDecl>,
    /// Methods of generic records and unions, by type and name.
    pub(crate) generic_methods: HashMap<(DefId, String), &'a ast::FnDecl>,
    /// Instances made so far: (function or owner type, name, type
    /// arguments). Methods have no definition of their own, so the name
    /// tells them apart.
    pub(crate) instances: HashMap<(DefId, String, Vec<Type>), FuncId>,
    /// The type arguments of each function instance, by function.
    pub(crate) fn_subst: HashMap<FuncId, HashMap<DefId, Type>>,
    /// The type arguments in effect: generic parameter → type.
    pub(crate) subst: HashMap<DefId, Type>,
    /// The error parameters of each instance that stand in for an error
    /// never made (`lower_generic.rs`), and those of the one being made.
    pub(crate) fn_stand_ins: HashMap<FuncId, Vec<DefId>>,
    pub(crate) stand_ins: Vec<DefId>,
    /// Names brought in by `use std.<module>`, and the module (spec 14).
    pub(crate) std_modules: HashMap<DefId, String>,
    pub(crate) agents: crate::lower_agents::Agents<'a>,
    /// Names a nested pattern on a part that may be missing gives values
    /// to in either branch (`lower_destructure.rs`): their locals.
    pub(crate) dest_finals: HashMap<DefId, Local>,
    /// A call that can fail with `inout` parameters, just lowered: its
    /// values go back only when it succeeds (`lower_inout.rs`).
    /// The record types whose `drop` the program registers (`lower_drops.rs`).
    pub(crate) drop_records: Vec<u32>,
    pub(crate) inout_pending: Option<crate::lower_inout::Pending<'a>>,
    /// Views of an `Error` being matched as each type its arms name
    /// (`lower_match.rs`): matched local, union, view local, union type.
    pub(crate) error_views: Vec<(Local, DefId, Local, Type)>,
    /// The source loops being lowered, innermost last, for labeled
    /// `break` and `continue` (`lower_labels.rs`).
    pub(crate) loop_frames: Vec<crate::lower_labels::LoopFrame>,
    /// The blocks given to a call being lowered (`lower_scoped.rs`): the
    /// local holding each one's depth, and how many loops were open.
    pub(crate) scoped_depths: Vec<(Local, usize)>,
}

/// Lowers a checked file, or reports the first feature the native
/// compiler doesn't support yet.
pub fn lower(c: &Compiled) -> Result<Program, Unsupported> {
    lower_with(c, &[])
}

/// Lowers a checked file, keeping the top-level functions `exports` too,
/// for a library (decision 0026).
pub fn lower_with(c: &Compiled, exports: &[&str]) -> Result<Program, Unsupported> {
    let decl = c
        .resolution
        .defs
        .iter()
        .enumerate()
        .filter_map(|(id, d)| d.span.map(|s| (s.start, id)))
        .collect();
    let mut l = Lowerer {
        c,
        decl,
        fn_ids: HashMap::new(),
        process_entries: Vec::new(),
        method_ids: HashMap::new(),
        iface_shims: HashMap::new(),
        fns: Vec::new(),
        shapes: Shapes::default(),
        locals: Vec::new(),
        local_ids: HashMap::new(),
        ret: Type::Unit,
        result: None,
        ensures: &[],
        self_local: None,
        invariant_of: None,
        its: Vec::new(),
        error_views: Vec::new(),
        loop_frames: Vec::new(),
        scoped_depths: Vec::new(),
        ready: HashMap::new(),
        wrappers: HashMap::new(),
        adapters: HashMap::new(),
        pending_error: None,
        generic_fns: HashMap::new(),
        generic_methods: HashMap::new(),
        instances: HashMap::new(),
        fn_subst: HashMap::new(),
        subst: HashMap::new(),
        fn_stand_ins: HashMap::new(),
        stand_ins: Vec::new(),
        std_modules: HashMap::new(),
        agents: Default::default(),
        dest_finals: HashMap::new(),
        inout_pending: None,
        drop_records: Vec::new(),
    };
    l.std_modules = crate::lower_std::std_modules(c, &l.decl);
    let mut top_level = Vec::new();
    for item in &c.file.items {
        match &item.kind {
            // A tag only checks text while the program is compiled.
            ItemKind::Literal(_) => {}
            ItemKind::Fn(f) => {
                if let Some(&def) = l.decl.get(&f.name.span.start) {
                    if f.generics.is_empty() {
                        l.fn_ids.insert(def, l.fns.len() as FuncId);
                        l.fns.push((Some(f), None));
                    } else {
                        l.generic_fns.insert(def, f);
                    }
                }
            }
            ItemKind::Type(t) => {
                if let (ast::TypeBody::Record(r), Some(&def)) =
                    (&t.body, l.decl.get(&t.name.span.start))
                {
                    l.add_methods(&r.methods, Owner::Adt(def));
                }
            }
            ItemKind::Extend(e) => {
                let TypeKind::Named { path, .. } = &e.target.kind else {
                    continue;
                };
                let first = &path.segments[0];
                let Some(&def) = c.resolution.uses.get(&first.span.start) else {
                    continue;
                };
                let owner = match c.resolution.defs[def].kind {
                    DefKind::Builtin => Owner::Builtin(first.name.clone()),
                    _ => Owner::Adt(def),
                };
                l.add_methods(&e.methods, owner);
            }
            ItemKind::Stmt(s) => top_level.push(s),
            ItemKind::Use(_) | ItemKind::Interface(_) => {}
            ItemKind::Capability(cap) => {
                if let Some(&def) = l.decl.get(&cap.name.span.start) {
                    l.agents.caps.push((def, cap));
                }
            }
            ItemKind::Agent(a) => l.add_agent(a),
            // `test` blocks are left out of a build (spec 11.8).
            ItemKind::Test(_) => {}
        }
    }
    // Lowering a body can ask for new instances of generic functions,
    // and lambdas and tasks become functions, all added to the end of
    // `fns`.
    let (mut functions, mut failed) = (Vec::new(), Vec::new());
    collect_functions(&mut l, &mut functions, &mut failed)?;
    let main = l
        .fns
        .iter()
        .position(|(f, owner)| owner.is_none() && f.is_some_and(|f| f.name.name == "main"));
    let entry = match main {
        Some(i) => i as FuncId,
        None => {
            // Its id comes first: lambdas and tasks in it take later ones.
            let id = l.fns.len() as FuncId;
            l.fns.push((None, None));
            l.start_function(Type::Unit);
            let mut body = ir::Block::default();
            for s in top_level {
                l.stmt(s, &mut body.stmts)?;
            }
            l.ready.insert(
                id,
                ir::Function {
                    name: "main".into(),
                    params: Vec::new(),
                    ret: Type::Unit,
                    locals: l.locals.clone(),
                    body,
                    line: 1,
                    // Top-level code may use `try` (spec 1.3).
                    fails: true,
                    external: None,
                },
            );
            collect_functions(&mut l, &mut functions, &mut failed)?;
            id
        }
    };
    // How tasks run, and each `Drop` type's `drop`, first (9.8, 6.1).
    let mut registers = l.program_start()?;
    // A child of `spawn process` runs its call instead (9.8).
    registers.extend(l.process_starts());
    collect_functions(&mut l, &mut functions, &mut failed)?;
    // Generic `Drop` types' instances are known now (6.1).
    loop {
        let more = l.generic_drop_registrations()?;
        if more.is_empty() {
            break;
        }
        registers.extend(more);
        collect_functions(&mut l, &mut functions, &mut failed)?;
    }
    if let Some(f) = functions.get_mut(entry as usize) {
        f.body.stmts.splice(0..0, registers);
    }
    // A library's exports (decision 0026) are kept, as `main` is.
    let roots: Vec<FuncId> = exports.iter().filter_map(|n| l.top_level_fn(n)).collect();
    let mut kept = vec![entry];
    kept.extend(&roots);
    crate::collect::none_reached(&functions, &kept, failed)?;
    let (functions, kept, quiet_from, notes) = crate::passes::finish(&mut l, c, functions, &kept);
    let exports = kept[1..]
        .iter()
        .copied()
        .zip(exports.iter().map(|n| n.to_string()));
    Ok(Program {
        // `first line<TAB>path`, one line per file: runtime messages map a
        // line of the program back to its file.
        path: c
            .files
            .iter()
            .map(|f| format!("{}\t{}", f.first_line, kavach_driver::display_path(&f.path)))
            .collect::<Vec<_>>()
            .join("\n"),
        functions,
        quiet_from,
        entry: kept[0],
        exports: exports.collect(),
        shapes: l.shapes.list,
        notes: notes.into_iter().map(|(l, n)| (l, n.to_string())).collect(),
    })
}

impl Program {
    /// The file, and the line in it, of line `line` of the whole program.
    pub fn location(&self, line: u32) -> (&str, u32) {
        let mut files = self.path.lines().filter_map(|l| {
            let (first, path) = l.split_once('\t')?;
            Some((first.parse::<u32>().ok()?, path))
        });
        match files.rfind(|(first, _)| *first <= line) {
            Some((first, path)) => (path, line - first + 1),
            None => ("?", line),
        }
    }
}

impl<'a> Lowerer<'a> {
    fn add_methods(&mut self, methods: &'a [ast::Item], owner: Owner) {
        for m in methods {
            if let ItemKind::Fn(f) = &m.kind {
                // A generic type's methods, and methods with type parameters
                // of their own, are made per set of type arguments, when
                // called (`lower_generic.rs`).
                if let Owner::Adt(def) = &owner
                    && (!self.type_generics(*def).is_empty() || !f.generics.is_empty())
                {
                    self.generic_methods.insert((*def, f.name.name.clone()), f);
                    continue;
                }
                self.method_ids.insert(
                    (owner.clone(), f.name.name.clone()),
                    self.fns.len() as FuncId,
                );
                self.fns.push((Some(f), Some(owner.clone())));
            }
        }
    }

    /// The function declared at the top level of the program's own files
    /// as `name`.
    fn top_level_fn(&self, name: &str) -> Option<FuncId> {
        let res = &self.c.resolution;
        self.fn_ids
            .iter()
            .find(|(d, _)| {
                let def = &res.defs[**d];
                def.name == name && def.span.is_some_and(|s| self.c.is_own(s.start))
            })
            .map(|(_, f)| *f)
    }

    pub(crate) fn line(&self, span: ast::Span) -> u32 {
        self.c.line_of(span.start) as u32
    }

    pub(crate) fn unsupported(&self, feature: impl Into<String>, span: ast::Span) -> Unsupported {
        Unsupported {
            feature: feature.into(),
            line: self.line(span),
        }
    }

    pub(crate) fn text(&self, span: ast::Span) -> &'a str {
        span.text(&self.c.src)
    }

    pub(crate) fn new_local(&mut self, ty: Type) -> Local {
        self.locals.push(ty);
        (self.locals.len() - 1) as Local
    }

    pub(crate) fn bind(&mut self, id: &ast::Ident, local: Local) {
        if let Some(&def) = self.decl.get(&id.span.start) {
            self.local_ids.insert(def, local);
        }
    }

    pub(crate) fn start_function(&mut self, ret: Type) {
        self.locals.clear();
        self.local_ids.clear();
        self.ret = ret;
        self.result = None;
        self.ensures = &[];
        self.self_local = None;
    }

    /// The type a method's `self` has.
    pub(crate) fn owner_type(&mut self, owner: &Owner, span: ast::Span) -> Lowered<Type> {
        match owner {
            Owner::Adt(def) => {
                // In an instance of a generic type's method, `self` has
                // the instance's type arguments.
                let args = self
                    .type_generics(*def)
                    .iter()
                    .map(|g| self.subst.get(g).copied())
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| self.unsupported("this method", span))?;
                self.adt_instance(*def, args, span)
            }
            Owner::Builtin(name) if name == "String" => Ok(self.str_type()),
            Owner::Builtin(name) => kavach_checker::Prim::from_name(name)
                .and_then(prim_type)
                .ok_or_else(|| self.unsupported(format!("methods on `{name}`"), span)),
        }
    }

    /// The declaration of program function `fid`.
    pub(crate) fn decl_of(&self, fid: FuncId) -> &'a ast::FnDecl {
        self.fns[fid as usize].0.expect("a declared function")
    }

    pub(crate) fn local_expr(&self, local: Local, line: u32) -> ir::Expr {
        ir::Expr {
            kind: ir::ExprKind::Local(local),
            ty: self.locals[local as usize],
            line,
        }
    }

    /// `expr` as type `want`: a `T` where a `T?` is expected becomes a
    /// present optional (spec 2.7), and a value where an interface is
    /// expected becomes an interface box (spec 2.15).
    pub(crate) fn coerce(&mut self, expr: ir::Expr, want: Type) -> Lowered<ir::Expr> {
        match self.shapes.shape(want) {
            Some(Shape::Optional(inner)) if *inner == expr.ty => {
                let line = expr.line;
                Ok(ir::Expr {
                    kind: ir::ExprKind::Some(Box::new(expr)),
                    ty: want,
                    line,
                })
            }
            Some(Shape::Iface { .. }) if expr.ty != want => self.iface_value(expr, want),
            // A function that can't fail where one that can is wanted.
            Some(Shape::Func(_, _, Some(_))) if self.fn_error(expr.ty).is_none() => {
                self.never_failing(expr, want)
            }
            _ => Ok(expr),
        }
    }

    /// Lowers `b` into `out`. With `want_value`, the last expression becomes
    /// the block's value.
    pub(crate) fn block_into(
        &mut self,
        b: &'a ast::Block,
        want_value: bool,
        out: &mut ir::Block,
    ) -> Lowered<()> {
        for (i, s) in b.stmts.iter().enumerate() {
            match (&s.kind, want_value && i + 1 == b.stmts.len()) {
                // `return`, `break` and `continue` leave instead of giving a
                // value (a `catch` handler that returns early).
                (StmtKind::Expr(e), true) if !leaves(e) => {
                    out.value = Some(Box::new(self.expr(e)?))
                }
                _ => self.stmt(s, &mut out.stmts)?,
            }
        }
        Ok(())
    }

    /// The function's `ensure` checks, on `result` (spec 7.2).
    pub(crate) fn ensure_checks(&mut self, out: &mut Vec<Stmt>) -> Lowered<()> {
        for e in self.ensures {
            let cond = self.expr(e)?;
            out.push(Stmt::Check {
                cond,
                message: format!("contract failed: ensure {}", self.text(e.span)),
                line: self.line(e.span),
            });
        }
        Ok(())
    }

    pub(crate) fn is_builtin(&self, callee: &ast::Expr, name: &str) -> bool {
        let ExprKind::Name(id) = &callee.kind else {
            return false;
        };
        let res = &self.c.resolution;
        id.name == name
            && res
                .uses
                .get(&id.span.start)
                .is_some_and(|d| res.defs[*d].kind == DefKind::Builtin)
    }

    pub(crate) fn local(&self, id: &ast::Ident) -> Lowered<Local> {
        let def = self.c.resolution.uses.get(&id.span.start);
        match def.map(|d| (*d, self.c.resolution.defs[*d].kind)) {
            Some((_, DefKind::Result)) => self
                .result
                .ok_or_else(|| self.unsupported("`result` here", id.span)),
            Some((d, _)) => self
                .local_ids
                .get(&d)
                .copied()
                .ok_or_else(|| self.unsupported(format!("the value `{}`", id.name), id.span)),
            None => Err(self.unsupported(format!("the name `{}`", id.name), id.span)),
        }
    }
}

/// Whether `e` leaves the block instead of giving it a value.
pub(crate) fn leaves(e: &ast::Expr) -> bool {
    matches!(
        e.kind,
        ExprKind::Return(_) | ExprKind::Break(_) | ExprKind::Continue(_)
    )
}
