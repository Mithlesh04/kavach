//! The checker's state and the helpers every part uses: errors, type
//! display, coercion and mismatch messages.

use crate::Options;
use crate::infer::{Infer, VarKind};
use crate::ty::Ty;
use kavach_ast::{Expr, Span};
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
    /// `-> secret T` (spec 8.7).
    pub(crate) ret_secret: bool,
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
#[derive(Clone)]
pub(crate) struct Ctx {
    pub(crate) name: String,
    pub(crate) ret: Ty,
    /// Whether it may return a secret (`-> secret T`, spec 8.7).
    pub(crate) ret_secret: bool,
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
    /// Functions of C libraries (`extern c`, spec 10.7), called only
    /// inside `unsafe`; for one ending in `...`, its own parameters' count.
    pub(crate) c_fns: HashMap<DefId, Option<usize>>,
    /// How many `unsafe` blocks the code being checked is inside.
    pub(crate) in_unsafe: u32,
    pub(crate) types: HashMap<DefId, TypeInfo>,
    /// `type Handler = (on: fn(Int) -> Int)`: the anonymous type each
    /// alias stands for (spec 2.14).
    pub(crate) aliases: HashMap<DefId, Ty>,
    /// Methods of records, unions, interfaces, capabilities and agents.
    pub(crate) methods: HashMap<DefId, HashMap<String, FnSig>>,
    /// Methods of records, unions and `extend`s not marked `pub` (10.1).
    pub(crate) private_methods: HashSet<DefId>,
    /// Methods added to built-in types with `extend` (spec 2.5), by type name.
    pub(crate) ext_methods: HashMap<String, HashMap<String, FnSig>>,
    /// The type each method added to lists, maps, sets and rings was
    /// written for (`[T]`), by kind and method: matched at each call.
    pub(crate) ext_targets: HashMap<(String, String), Ty>,
    /// A script's top-level constants, and those of them that stay its
    /// statements, as they use a capability or `try` (`module_consts.rs`).
    pub(crate) script_candidates: HashSet<DefId>,
    pub(crate) script_consts: HashSet<DefId>,
    /// `std.reflect`'s answers by call span (`reflect.rs`), and the
    /// methods `extend` added.
    pub(crate) reflect_texts: HashMap<(u32, u32), String>,
    pub(crate) reflect_bools: HashMap<(u32, u32), bool>,
    pub(crate) extended_methods: HashSet<DefId>,
    pub(crate) implements: HashMap<DefId, HashSet<DefId>>,
    /// The record types that implement `Drop` (`drops.rs`).
    pub(crate) drop_types: HashSet<DefId>,
    /// Each type's operators (spec 2.16, `user_ops.rs`): by type and
    /// method, the other operand's type, the result's, and where.
    pub(crate) operators: HashMap<(DefId, &'static str), (Ty, Ty, Span)>,
    /// Types given for number-bounded parameters, checked once every
    /// literal's type is known (spec 2.9).
    pub(crate) number_checks: Vec<(DefId, Ty, Span)>,
    /// Each type argument of a record made here, to check something
    /// filled it (2.9).
    pub(crate) filled_checks: Vec<(DefId, Ty, Span)>,
    /// Each method's `where` bounds (spec 2.9), by its type and name: the
    /// type parameter, and whether it must be `Floating`.
    pub(crate) wheres: HashMap<(DefId, String), Vec<(DefId, bool)>>,
    /// The `where` bounds of the calls seen, checked once literals'
    /// types are known: the bound, the type given, the call and method.
    pub(crate) where_checks: Vec<(DefId, bool, Ty, Span, String)>,
    /// Each type parameter's operator bounds, as their methods (spec 2.16).
    pub(crate) param_ops: HashMap<DefId, Vec<&'static str>>,
    /// The operators used on a program's own types, by `(left start,
    /// right end)`, with the method each calls; `-x` by `x`'s span; and
    /// `x op= v` by `(x start, v end)`.
    pub(crate) user_ops: HashMap<(u32, u32), &'static str>,
    pub(crate) user_negs: HashSet<(u32, u32)>,
    pub(crate) user_assigns: HashMap<(u32, u32), &'static str>,
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
    /// While a built-in method's arguments are checked: the errors of the
    /// functions given to it, which the call passes on (spec 5.2).
    pub(crate) rethrow: Option<Vec<Ty>>,
    /// Whether the lambda being checked has passed an error on with
    /// `try` or `fail` (spec 3.3).
    pub(crate) lambda_failed: bool,
    /// The value being fitted to an expected type, and the `uses` limits
    /// of function types that named functions and lambdas were given to,
    /// checked once capabilities are known (spec 3.4).
    pub(crate) fitting: Option<crate::fn_values::Made>,
    pub(crate) uses_limits: Vec<(crate::fn_values::Made, Vec<String>)>,
    /// The lambdas being checked, innermost last, and what each needs.
    pub(crate) lambdas: Vec<Span>,
    pub(crate) lambda_needs: HashMap<(u32, u32), Vec<crate::capabilities::Need>>,
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
    /// Each float literal's span and text, to check it fits its type.
    pub(crate) float_texts: Vec<(Span, String)>,
    /// Constants typed `BigInt`, with their digits (`constants.rs`).
    pub(crate) big_constants: HashMap<(u32, u32), String>,
    /// Constants with an operator, exactly, for those that turn out to be
    /// floats (`constants.rs`).
    pub(crate) exact_constants: Vec<((u32, u32), crate::constants::Exact)>,
    /// The `gpu fn`s (spec 15), and `map` calls that may run on the GPU:
    /// (span, item type, captured names), settled once types are known.
    pub(crate) gpu_fns: HashSet<DefId>,
    /// Each `gpu fn`'s stage: a `map`'s, or a draw's (spec 15.3).
    pub(crate) gpu_stages: HashMap<DefId, kavach_ast::GpuStage>,
    /// Each `gpu.pipeline(v, f)` call's vertex and fragment fns, by span.
    pub(crate) pipelines: HashMap<(u32, u32), (DefId, DefId)>,
    pub(crate) gpu_candidates: Vec<crate::gpu::GpuCandidate>,
    /// Every checked expression and its (possibly still unknown) type.
    pub(crate) expr_types: Vec<(Span, Ty)>,
    /// `Shared.update` calls, whose result may be left unused (spec 6.2).
    pub(crate) updates: HashSet<(u32, u32)>,
    /// Lambdas kept in a `let` whose parameters have no written types,
    /// with their type. The body is checked at the first call, once the
    /// argument types are known, or at the end of the function.
    pub(crate) deferred: HashMap<DefId, (Expr, Ty)>,
    /// The function whose body is being checked, and what each function
    /// body uses (`capabilities.rs`).
    pub(crate) current_fn: Option<DefId>,
    /// `json.of(v)` and `json.stringify(v)`: where, and `v`'s type; and each
    /// call of a generic function with its type arguments (`generic_json.rs`).
    pub(crate) json_writes: Vec<(Option<DefId>, Ty, Span)>,
    pub(crate) generic_calls: Vec<crate::generic_json::GenericCall>,
    /// Names brought in by `use std.<module>`, and the module (spec 14).
    pub(crate) std_modules: HashMap<DefId, String>,
    /// What each function uses, once worked out (`capabilities.rs`).
    pub(crate) fn_uses: HashMap<DefId, Vec<String>>,
    pub(crate) needs: HashMap<DefId, Vec<crate::capabilities::NeedAt>>,
    /// Each `test` block (spec 11.8): what it may use (`uses`, `with`),
    /// and what it uses; `current_test` is the one being checked.
    pub(crate) test_needs: Vec<crate::capabilities::TestNeeds>,
    pub(crate) current_test: Option<usize>,
    /// The type whose `extend` block is being checked: inside the type's
    /// own package, it sees the type's private fields (spec 10.1).
    pub(crate) extending: Option<DefId>,
    /// The tags for literals, and each literal written (spec 10.15).
    pub(crate) literal_tags: HashMap<DefId, crate::literals::LiteralTag>,
    pub(crate) literal_uses: Vec<crate::literals::LiteralUse>,
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
            private_methods: HashSet::new(),
            ext_methods: HashMap::new(),
            ext_targets: HashMap::new(),
            script_candidates: HashSet::new(),
            script_consts: HashSet::new(),
            reflect_texts: HashMap::new(),
            reflect_bools: HashMap::new(),
            extended_methods: HashSet::new(),
            implements: HashMap::new(),
            drop_types: HashSet::new(),
            operators: HashMap::new(),
            number_checks: Vec::new(),
            filled_checks: Vec::new(),
            wheres: HashMap::new(),
            where_checks: Vec::new(),
            param_ops: HashMap::new(),
            user_ops: HashMap::new(),
            user_negs: HashSet::new(),
            user_assigns: HashMap::new(),
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
            rethrow: None,
            lambda_failed: false,
            fitting: None,
            uses_limits: Vec::new(),
            lambdas: Vec::new(),
            lambda_needs: HashMap::new(),
            loops: Vec::new(),
            in_contract: false,
            in_example: false,
            record: None,
            secret: HashSet::new(),
            secret_defs: HashSet::new(),
            literals: Vec::new(),
            float_texts: Vec::new(),
            big_constants: HashMap::new(),
            exact_constants: Vec::new(),
            gpu_fns: HashSet::new(),
            gpu_stages: HashMap::new(),
            pipelines: HashMap::new(),
            gpu_candidates: Vec::new(),
            expr_types: Vec::new(),
            updates: HashSet::new(),
            deferred: HashMap::new(),
            current_fn: None,
            json_writes: Vec::new(),
            generic_calls: Vec::new(),
            std_modules: HashMap::new(),
            fn_uses: HashMap::new(),
            needs: HashMap::new(),
            test_needs: Vec::new(),
            current_test: None,
            extending: None,
            literal_tags: HashMap::new(),
            literal_uses: Vec::new(),
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
        if let (Ty::Fn(..), Ty::Fn(..)) = (&a, &e) {
            return self.fn_fits(&a, &e);
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
        self.fitting = Some(self.made(e));
        let fits = self.coerce(&actual, expected);
        self.fitting = None;
        if !fits {
            self.mismatch(e, expected, &actual);
        }
        actual
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
