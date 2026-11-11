//! The checker's state and the helpers every part uses: errors, type
//! display, coercion and mismatch messages.

use crate::Options;
use crate::infer::{Infer, VarKind};
use crate::ty::Ty;
use kavach_ast::{Expr, ExprKind, Span};
use kavach_resolver::{DefId, Diagnostic, Resolution};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub(crate) struct FieldInfo {
    pub(crate) name: String,
    pub(crate) ty: Ty,
    pub(crate) has_default: bool,
    pub(crate) secret: bool,
    pub(crate) is_pub: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct ParamInfo {
    pub(crate) name: String,
    pub(crate) ty: Ty,
    pub(crate) has_default: bool,
    /// `secret card: String` (spec 8.7).
    pub(crate) secret: bool,
    /// `inout n: Int` (spec 3.9).
    pub(crate) inout: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct FnSig {
    pub(crate) generics: Vec<(DefId, Vec<DefId>)>,
    pub(crate) params: Vec<ParamInfo>,
    pub(crate) ret: Ty,
    pub(crate) errors: Option<Ty>,
    pub(crate) uses: Vec<String>,
    pub(crate) has_self: bool,
    /// The function's own definition, for following calls (spec 8.1).
    pub(crate) def: Option<DefId>,
}

#[derive(Clone, Debug)]
pub(crate) struct VariantInfo {
    pub(crate) name: String,
    pub(crate) def: DefId,
    pub(crate) fields: Vec<FieldInfo>,
    /// An enum variant's number (spec 2.6).
    pub(crate) value: Option<i128>,
}

#[derive(Clone, Debug)]
pub(crate) enum Shape {
    Record(Vec<FieldInfo>),
    Union(Vec<VariantInfo>),
    Other,
}

#[derive(Clone, Debug)]
pub(crate) struct TypeInfo {
    pub(crate) generics: Vec<DefId>,
    pub(crate) shape: Shape,
}

/// The function being checked.
pub(crate) struct Ctx {
    pub(crate) name: String,
    pub(crate) ret: Ty,
    pub(crate) errors: Option<Ty>,
    pub(crate) self_ty: Option<Ty>,
}

pub(crate) struct Checker<'a> {
    pub(crate) res: &'a Resolution,
    pub(crate) opts: Options,
    pub(crate) inf: Infer,
    pub(crate) errors: Vec<Diagnostic>,
    /// Things that work but are probably not what was meant.
    pub(crate) warnings: Vec<Diagnostic>,
    /// Declaration span start → definition, for every declared name.
    pub(crate) decl: HashMap<u32, DefId>,
    pub(crate) fns: HashMap<DefId, FnSig>,
    /// Functions of C libraries (`extern c`, spec 10.7): called only
    /// inside `unsafe`.
    pub(crate) c_fns: std::collections::HashSet<DefId>,
    /// How many `unsafe` blocks the code being checked is inside.
    pub(crate) in_unsafe: u32,
    pub(crate) types: HashMap<DefId, TypeInfo>,
    /// `type Handler = (on: fn(Int) -> Int)`: the anonymous type each
    /// alias stands for (spec 2.14).
    pub(crate) aliases: HashMap<DefId, Ty>,
    /// Methods of records, unions, interfaces, capabilities and agents.
    pub(crate) methods: HashMap<DefId, HashMap<String, FnSig>>,
    /// Methods added to built-in types with `extend` (spec 2.5), by type name.
    pub(crate) ext_methods: HashMap<String, HashMap<String, FnSig>>,
    pub(crate) implements: HashMap<DefId, HashSet<DefId>>,
    /// The record types that implement `Drop` (`drops.rs`).
    pub(crate) drop_types: HashSet<DefId>,
    /// The variant each variant pattern names, by the span start of its
    /// last name: settled from the matched value's type (`patterns.rs`).
    pub(crate) pattern_variants: HashMap<u32, DefId>,
    /// Interfaces built-in types implement with `extend`, by type name.
    pub(crate) ext_implements: HashMap<String, HashSet<DefId>>,
    /// The declared capabilities each agent `can` use, by name: what its
    /// `start` takes (spec 8.8).
    pub(crate) agent_needs: HashMap<DefId, std::collections::BTreeMap<String, DefId>>,
    /// Interface bounds of each generic parameter.
    pub(crate) bounds: HashMap<DefId, Vec<DefId>>,
    /// The type parameters of each generic interface, so a method called
    /// on a `Getter<Int>` value gives an `Int` (spec 2.15).
    pub(crate) iface_generics: HashMap<DefId, Vec<DefId>>,
    /// The type arguments of each interface a type implements, by the
    /// type (or built-in type's name) and the interface: `type Word:
    /// Getter<String>` fits a `Getter<String>`, never a `Getter<Int>`.
    pub(crate) impl_args: HashMap<(String, DefId), Vec<Ty>>,
    pub(crate) locals: HashMap<DefId, Ty>,
    pub(crate) ctx: Option<Ctx>,
    /// The value of each enclosing dot lambda (`.status`).
    pub(crate) its: Vec<Ty>,
    /// Enclosing `arena` blocks (spec 6.3).
    pub(crate) arenas: Vec<Span>,
    /// The lambdas and `spawn` blocks being checked, innermost last: where
    /// each body starts, and what it is. Outside variables are copies
    /// inside them (spec 3.3, 9.5).
    pub(crate) copies: Vec<(u32, &'static str)>,
    /// How deep the checker is inside the function given to `update`,
    /// which may not wait (spec 6.2).
    pub(crate) updating: u32,
    /// The loops around the code being checked, innermost last, with
    /// their labels (spec 4.6). A function's or lambda's body starts with
    /// none: a `break` can't leave it.
    pub(crate) loops: Vec<Option<String>>,
    /// Inside `require`, `ensure`, `example` or `invariant` (spec 7.5).
    pub(crate) in_contract: bool,
    /// Inside `example`: a call that can fail needs no `try` (spec 7.6).
    pub(crate) in_example: bool,
    /// The record whose `invariant` is being checked.
    pub(crate) record: Option<DefId>,
    /// Field accesses that read a `secret` field (spec 2.11), and calls
    /// of `env` (spec 10.13).
    pub(crate) secret: HashSet<(u32, u32)>,
    /// Parameters marked `secret`, and variables set from a secret: their
    /// values are secrets too (spec 8.7).
    pub(crate) secret_defs: HashSet<DefId>,
    /// Every number literal and its (possibly still unknown) type.
    pub(crate) literals: Vec<(u32, Ty)>,
    /// Constants typed `BigInt`, with their digits (`constants.rs`).
    pub(crate) big_constants: HashMap<(u32, u32), String>,
    /// Constants with an operator, exactly, for those that turn out to be
    /// floats (`constants.rs`).
    pub(crate) exact_constants: Vec<((u32, u32), crate::constants::Exact)>,
    /// The `gpu fn`s (spec 15), and `map` calls that may run on the GPU:
    /// (span, item type, captured names), settled once types are known.
    pub(crate) gpu_fns: HashSet<DefId>,
    pub(crate) gpu_candidates: Vec<crate::gpu::GpuCandidate>,
    /// Every checked expression and its (possibly still unknown) type.
    pub(crate) expr_types: Vec<(Span, Ty)>,
    /// Lambdas kept in a `let` whose parameters have no written types,
    /// with their type. The body is checked at the first call, once the
    /// argument types are known, or at the end of the function.
    pub(crate) deferred: HashMap<DefId, (Expr, Ty)>,
    /// The function whose body is being checked, and what each function
    /// body uses (`capabilities.rs`).
    pub(crate) current_fn: Option<DefId>,
    /// Names brought in by `use std.<module>`, and the module (spec 14).
    pub(crate) std_modules: HashMap<DefId, String>,
    /// What each function uses, once worked out (`capabilities.rs`).
    pub(crate) fn_uses: HashMap<DefId, Vec<String>>,
    pub(crate) needs: HashMap<DefId, Vec<crate::capabilities::NeedAt>>,
}

impl<'a> Checker<'a> {
    pub(crate) fn new(res: &'a Resolution, opts: Options) -> Self {
        let decl = res
            .defs
            .iter()
            .enumerate()
            .filter_map(|(id, d)| d.span.map(|s| (s.start, id)))
            .collect();
        Checker {
            res,
            opts,
            inf: Infer::default(),
            errors: Vec::new(),
            warnings: Vec::new(),
            decl,
            fns: HashMap::new(),
            c_fns: Default::default(),
            in_unsafe: 0,
            types: HashMap::new(),
            aliases: HashMap::new(),
            methods: HashMap::new(),
            ext_methods: HashMap::new(),
            implements: HashMap::new(),
            drop_types: HashSet::new(),
            pattern_variants: HashMap::new(),
            ext_implements: HashMap::new(),
            agent_needs: HashMap::new(),
            bounds: HashMap::new(),
            iface_generics: HashMap::new(),
            impl_args: HashMap::new(),
            locals: HashMap::new(),
            ctx: None,
            its: Vec::new(),
            arenas: Vec::new(),
            copies: Vec::new(),
            updating: 0,
            loops: Vec::new(),
            in_contract: false,
            in_example: false,
            record: None,
            secret: HashSet::new(),
            secret_defs: HashSet::new(),
            literals: Vec::new(),
            big_constants: HashMap::new(),
            exact_constants: Vec::new(),
            gpu_fns: HashSet::new(),
            gpu_candidates: Vec::new(),
            expr_types: Vec::new(),
            deferred: HashMap::new(),
            current_fn: None,
            std_modules: HashMap::new(),
            fn_uses: HashMap::new(),
            needs: HashMap::new(),
        }
    }

    pub(crate) fn error(&mut self, span: Span, message: impl Into<String>) {
        self.errors.push(Diagnostic {
            message: message.into(),
            span,
        });
    }

    /// `what` works on raw memory: only inside `unsafe { … }` (spec 6.4).
    pub(crate) fn need_unsafe(&mut self, what: &str, span: Span) {
        if self.in_unsafe == 0 {
            self.error(
                span,
                format!("{what} works on raw memory: use it inside `unsafe {{ … }}` (spec 6.4)"),
            );
        }
    }

    pub(crate) fn fresh(&mut self) -> Ty {
        self.inf.fresh(VarKind::Any)
    }

    pub(crate) fn def_name(&self, def: DefId) -> &str {
        &self.res.defs[def].name
    }

    /// A type as people write it, for messages.
    pub(crate) fn show(&self, ty: &Ty) -> String {
        let list = |ts: &[Ty]| {
            ts.iter()
                .map(|t| self.show(t))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let args = |ts: &[Ty]| {
            if ts.is_empty() {
                String::new()
            } else {
                format!("<{}>", list(ts))
            }
        };
        match self.inf.resolve(ty) {
            Ty::Prim(p) => p.name().to_string(),
            Ty::Unit => "nothing".to_string(),
            Ty::Never => "never".to_string(),
            Ty::Error => "?".to_string(),
            Ty::List(t) => format!("[{}]", self.show(&t)),
            Ty::Set(t) => format!("Set<{}>", self.show(&t)),
            Ty::Map(k, v) => format!("[{}: {}]", self.show(&k), self.show(&v)),
            Ty::Optional(t) => format!("{}?", self.show(&t)),
            Ty::Range(t) => format!("Range<{}>", self.show(&t)),
            Ty::Tuple(ts) => format!("({})", list(&ts)),
            Ty::Anon(fs) => format!(
                "({})",
                fs.iter()
                    .map(|(n, t)| format!("{n}: {}", self.show(t)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Ty::Fn(ps, r) => format!("fn({}) -> {}", list(&ps), self.show(&r)),
            Ty::Adt(d, ts) => format!("{}{}", self.def_name(d), args(&ts)),
            Ty::Builtin(n, ts) => format!("{n}{}", args(&ts)),
            Ty::Param(d) => self.def_name(d).to_string(),
            Ty::Result(o, _) => self.show(&o),
            Ty::Var(v) => match self.inf.kind(v) {
                VarKind::IntLiteral => "Int".to_string(),
                VarKind::FloatLiteral => "Float".to_string(),
                VarKind::Any => "_".to_string(),
            },
        }
    }

    /// `ty` with every variable replaced, number literals defaulting to
    /// `Int` and `Float`, and unknowns becoming `Error`.
    pub(crate) fn defaulted(&self, ty: &Ty) -> Ty {
        let resolved = self.inf.resolve(ty);
        self.default_vars(&resolved)
    }

    fn default_vars(&self, ty: &Ty) -> Ty {
        let d = |t: &Ty| Box::new(self.default_vars(t));
        let all = |ts: &[Ty]| ts.iter().map(|t| self.default_vars(t)).collect();
        match ty {
            Ty::Var(v) => match self.inf.kind(*v) {
                VarKind::IntLiteral => Ty::Prim(crate::ty::Prim::Int),
                VarKind::FloatLiteral => Ty::Prim(crate::ty::Prim::Float),
                VarKind::Any => Ty::Error,
            },
            Ty::List(t) => Ty::List(d(t)),
            Ty::Set(t) => Ty::Set(d(t)),
            Ty::Optional(t) => Ty::Optional(d(t)),
            Ty::Range(t) => Ty::Range(d(t)),
            Ty::Map(k, v) => Ty::Map(d(k), d(v)),
            Ty::Result(o, e) => Ty::Result(d(o), d(e)),
            Ty::Tuple(ts) => Ty::Tuple(all(ts)),
            Ty::Anon(fs) => Ty::Anon(
                fs.iter()
                    .map(|(n, t)| (n.clone(), self.default_vars(t)))
                    .collect(),
            ),
            Ty::Fn(ps, r) => Ty::Fn(all(ps), d(r)),
            Ty::Adt(id, ts) => Ty::Adt(*id, all(ts)),
            Ty::Builtin(n, ts) => Ty::Builtin(n, all(ts)),
            other => other.clone(),
        }
    }

    /// Whether a value of type `actual` fits where `expected` is needed.
    /// A `T` fits where a `T?` is expected (spec 2.7).
    pub(crate) fn coerce(&mut self, actual: &Ty, expected: &Ty) -> bool {
        let (a, e) = (self.inf.shallow(actual), self.inf.shallow(expected));
        // A value of a type that implements the interface fits where the
        // interface is expected (spec 2.15).
        if self.implements_interface(&a, &e) {
            return self.iface_args_fit(&a, &e);
        }
        // A number literal fits an interface its own type implements:
        // `4` is an `Int`, `2.5` a `Float`.
        if let Ty::Var(v) = &a {
            let own = match self.inf.kind(*v) {
                crate::infer::VarKind::IntLiteral => Some(crate::Prim::Int),
                crate::infer::VarKind::FloatLiteral => Some(crate::Prim::Float),
                crate::infer::VarKind::Any => None,
            };
            if let Some(p) = own.map(Ty::Prim)
                && self.implements_interface(&p, &e)
            {
                self.inf.unify(&a, &p);
                return self.iface_args_fit(&p, &e);
            }
        }
        if let (Ty::Optional(x), Ty::Optional(y)) = (&a, &e)
            && self.implements_interface(x, y)
        {
            return self.iface_args_fit(x, y);
        }
        if let Ty::Optional(inner) = &e {
            // A number literal is never itself optional, so it fills the inner type.
            let literal = matches!(a, Ty::Var(v) if self.inf.kind(v) != crate::infer::VarKind::Any);
            if literal || !matches!(a, Ty::Optional(_) | Ty::Var(_)) {
                return self.inf.unify(&a, inner);
            }
        }
        self.inf.unify(&a, &e)
    }

    /// Whether `actual` is a type that implements the interface
    /// `expected` names (spec 2.15, decision 0029).
    pub(crate) fn implements_interface(&self, actual: &Ty, expected: &Ty) -> bool {
        let Ty::Adt(iface, _) = expected else {
            return false;
        };
        if self.res.defs[*iface].kind != kavach_resolver::DefKind::Interface {
            return false;
        }
        match actual {
            Ty::Adt(d, _) => {
                d != iface && self.implements.get(d).is_some_and(|s| s.contains(iface))
            }
            Ty::Prim(p) => self
                .ext_implements
                .get(p.name())
                .is_some_and(|s| s.contains(iface)),
            _ => false,
        }
    }

    /// Whether the interface type arguments `actual` implements fit
    /// those of `expected` (both found by `implements_interface`).
    fn iface_args_fit(&mut self, actual: &Ty, expected: &Ty) -> bool {
        let Ty::Adt(iface, want) = expected else {
            return true;
        };
        let (key, subst) = match actual {
            Ty::Adt(d, targs) => (impl_key_adt(*d), self.subst_of(*d, targs)),
            Ty::Prim(p) => (p.name().to_string(), HashMap::new()),
            _ => return true,
        };
        let Some(given) = self.impl_args.get(&(key, *iface)).cloned() else {
            return want.is_empty();
        };
        given.len() == want.len()
            && given.iter().zip(want).all(|(g, w)| {
                let g = self.inf.subst(g, &subst);
                self.inf.unify(&g, w)
            })
    }

    /// Checks `e` against `expected`, reporting a clear mismatch.
    pub(crate) fn check_expect(&mut self, e: &Expr, expected: &Ty) -> Ty {
        let actual = self.value(e, Some(expected));
        if !self.coerce(&actual, expected) {
            self.mismatch(e, expected, &actual);
        }
        actual
    }

    pub(crate) fn mismatch(&mut self, e: &Expr, expected: &Ty, actual: &Ty) {
        let (want, got) = (self.show(expected), self.show(actual));
        let (we, ga) = (self.inf.resolve(expected), self.inf.resolve(actual));
        let message = match (&we, &ga, &e.kind) {
            (_, _, ExprKind::None) => format!(
                "`{want}` can't hold `none`: write `{want}?` for a value that may be missing"
            ),
            (Ty::Prim(p), Ty::Prim(q), _) if p.is_numeric() && q.is_numeric() => format!(
                "there are no implicit number conversions: expected `{want}`, found `{got}`; convert with `{want}(…)`"
            ),
            (_, Ty::Unit, _) => format!("expected `{want}`, but this returns nothing"),
            (Ty::Builtin(a, x), Ty::Builtin(b, y), _)
                if *a == "Shared"
                    && *b == "Shared"
                    && matches!(x.first(), Some(Ty::Optional(t)) if Some(&**t) == y.first()) =>
            {
                format!(
                    "expected `{want}`, found `{got}`: every copy of a `Shared` sees one value, so its type can't change; make it as `{want}` from the start"
                )
            }
            _ => format!("expected `{want}`, found `{got}`"),
        };
        self.error(e.span, message);
    }

    /// Whether `ty` carries no value worth keeping.
    pub(crate) fn is_nothing(&self, ty: &Ty) -> bool {
        matches!(
            self.inf.shallow(ty),
            Ty::Unit | Ty::Never | Ty::Error | Ty::Var(_)
        )
    }

    /// `the call to add`-style description of an expression, for messages.
    pub(crate) fn describe(&self, e: &Expr) -> String {
        match &e.kind {
            ExprKind::Call { callee, .. } => match &callee.kind {
                ExprKind::Name(id) => format!("`{}`", id.name),
                ExprKind::Field { name, .. } => format!("`{}`", name.name),
                _ => "this call".to_string(),
            },
            _ => "this".to_string(),
        }
    }

    /// The type of a fresh instance of a record or union: `Pair<?, ?>`.
    pub(crate) fn instance(&mut self, def: DefId) -> (Ty, HashMap<DefId, Ty>) {
        let generics = self
            .types
            .get(&def)
            .map(|t| t.generics.clone())
            .unwrap_or_default();
        let subst: HashMap<DefId, Ty> = generics.iter().map(|g| (*g, self.fresh())).collect();
        let args = generics.iter().map(|g| subst[g].clone()).collect();
        (Ty::Adt(def, args), subst)
    }

    /// Maps a type's generic parameters to the arguments of `ty`.
    pub(crate) fn subst_of(&self, def: DefId, args: &[Ty]) -> HashMap<DefId, Ty> {
        let generics = match self.types.get(&def) {
            Some(t) => t.generics.clone(),
            None => self.iface_generics.get(&def).cloned().unwrap_or_default(),
        };
        generics.into_iter().zip(args.iter().cloned()).collect()
    }
}

/// The `impl_args` key of a declared type.
pub(crate) fn impl_key_adt(def: DefId) -> String {
    format!("#{def:?}")
}
