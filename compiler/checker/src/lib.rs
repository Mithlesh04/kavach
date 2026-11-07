//! The Kavach type checker (spec chapters 2–7): infers and checks types,
//! `match` coverage, error handling, unused and missing values, and the
//! safety rules for contracts, secrets, arenas and `unsafe`.
//!
//! It works on one resolved file. Capability inference and agent rules
//! (spec chapter 8) are checked in a later version.

mod agent_settings;
mod agents;
mod ai_models;
mod always_panics;
mod builtin_methods;
mod builtins;
mod c_functions;
mod calls;
mod capabilities;
mod chains;
mod checker;
mod collect;
mod conform;
mod constants;
mod conversions;
mod deprecated;
mod destructure;
mod drops;
mod enums;
mod exprs;
mod failures;
mod fields;
mod gpu;
mod gpu_fit;
mod gpu_walk;
mod infer;
mod inout;
mod items;
mod json;
mod lambdas;
mod method_names;
mod methods;
mod more_methods;
mod never_runs;
mod operators;
mod order;
mod patterns;
mod pointers;
mod processes;
mod stdlib;
mod ty;

pub use agent_settings::{AgentSetup, AuditSetup};
pub use ai_models::AiModel;
pub use json::json_literal_kind;
use kavach_ast::File;
pub use kavach_resolver::Diagnostic;
use kavach_resolver::{DefId, Resolution};
use std::collections::{HashMap, HashSet};
pub use ty::{Prim, Ty};

/// The units a whole number can be followed by to make a `Duration`
/// (`5.seconds`, spec 14.4), and their length in milliseconds.
pub const DURATION_UNITS: [(&str, i64); 5] = [
    ("ms", 1),
    ("seconds", 1_000),
    ("minutes", 60_000),
    ("hours", 3_600_000),
    ("days", 86_400_000),
];

#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    /// `unsafe = true` in `kavach.toml` (spec 6.4). Off by default.
    pub allow_unsafe: bool,
}

#[derive(Clone, Debug)]
pub struct Checked {
    pub errors: Vec<Diagnostic>,
    /// Code that works but is probably not what was meant.
    pub warnings: Vec<Diagnostic>,
    /// The type of every number literal, by the start offset of its token.
    /// Context decides it (spec 1.7); without context `1` is an `Int` and
    /// `0.5` a `Float`.
    pub literal_types: HashMap<u32, Prim>,
    /// The exact value, as digits, of each constant whose type is `BigInt`
    /// (spec 2.1), by its span `(start, end)`: worked out here, so code
    /// generation makes the number without doing the arithmetic again.
    pub big_constants: HashMap<(u32, u32), String>,
    /// Each constant with an operator whose type is a float, as the float
    /// nearest its exact value (decision 0043), by span.
    pub float_constants: HashMap<(u32, u32), f64>,
    /// The `map` calls that may run on the GPU (spec 15.2), by span.
    pub gpu_maps: HashSet<(u32, u32)>,
    /// The type of every checked expression, by its span `(start, end)`.
    /// Code generation reads these. Number literals without context are
    /// `Int` or `Float` here too.
    pub expr_types: HashMap<(u32, u32), Ty>,
    /// The capabilities each function uses, written or inferred (spec 8.1).
    pub uses: HashMap<DefId, Vec<String>>,
    /// The type of each field of every record and union variant, by the
    /// record's or variant's definition and the field's name: for
    /// decoding JSON (spec 2.12).
    pub field_types: HashMap<(DefId, String), Ty>,
    /// The type of every local variable, by its definition: for names a
    /// pattern takes (spec 1.12), whose types depend on what may be
    /// missing.
    pub local_types: HashMap<DefId, Ty>,
    /// The record types that implement `Drop` (spec 6.1).
    pub drop_types: HashSet<DefId>,
    /// The variables whose values are or hold a `Drop` value: they live
    /// until the end of their block, so they count as used.
    pub drop_locals: HashSet<DefId>,
    /// Each agent's `model`, by the agent's name (decision 0056).
    pub agent_models: HashMap<String, AiModel>,
    /// Each `ask`'s own `model:`, by the call's span.
    pub ask_models: HashMap<(u32, u32), AiModel>,
    /// Each agent's `settings`, by its name (decision 0060).
    pub agent_settings: HashMap<String, AgentSetup>,
}

/// The built-in methods a value of type `t` has (spec 14.2), for tools
/// such as code completion.
pub fn builtin_methods(t: &Ty, resolution: &Resolution) -> Vec<&'static str> {
    let mut checker = checker::Checker::new(resolution, Options::default());
    method_names::NAMES
        .iter()
        .copied()
        .filter(|name| checker.builtin_method(t, name).is_some())
        .collect()
}

/// Type-checks one parsed and resolved file.
pub fn check(file: &File, resolution: &Resolution, options: Options) -> Checked {
    let mut checker = checker::Checker::new(resolution, options);
    checker.collect(file);
    checker.check_conformance(file);
    checker.collect_uses(file);
    checker.check_capability_errors(file);
    checker.collect_gpu_fns(file);
    checker.check_file(file);
    checker.check_gpu_fns(file);
    checker.check_capabilities(file);
    checker.check_never_runs(file);
    checker.check_always_panics(file);
    checker.check_deprecated(file);
    let (agent_models, ask_models) = checker.ai_models(file);
    let agent_settings = checker.agent_settings(file);
    let literal_types = checker
        .literals
        .iter()
        .map(|(start, ty)| {
            let prim = match checker.inf.resolve(ty) {
                Ty::Prim(p) => p,
                Ty::Var(v) if checker.inf.kind(v) == infer::VarKind::FloatLiteral => Prim::Float,
                _ => Prim::Int,
            };
            (*start, prim)
        })
        .collect();
    let expr_types: HashMap<(u32, u32), Ty> = checker
        .expr_types
        .iter()
        .map(|(span, ty)| ((span.start, span.end), checker.defaulted(ty)))
        .collect();
    let float_constants = checker
        .exact_constants
        .iter()
        .filter(|(span, _)| {
            // `Float` only: a `Float32` constant keeps single precision.
            matches!(expr_types.get(span), Some(Ty::Prim(Prim::Float)))
        })
        .map(|(span, v)| (*span, v.to_f64()))
        .collect();
    let gpu_maps = checker
        .gpu_candidates
        .iter()
        .filter(|(span, item, captures, kind)| {
            let item = checker.defaulted(item);
            let fits = match kind {
                gpu::GpuCall::Map => gpu::gpu_item(&checker, &item),
                gpu::GpuCall::Sum => {
                    gpu::gpu_item(&checker, &item) && expr_types.get(span).is_some_and(gpu::whole32)
                }
                gpu::GpuCall::MinMax => gpu::whole32(&item),
                gpu::GpuCall::Filter => gpu::gpu_type(&item),
            };
            fits && captures.iter().all(|d| {
                let t = checker.locals.get(d).map(|t| checker.defaulted(t));
                t.is_some_and(|t| gpu::gpu_type(&t))
            })
        })
        .map(|(span, ..)| *span)
        .collect();
    let mut field_types = HashMap::new();
    for (def, info) in &checker.types {
        let mut add = |owner: DefId, fields: &[checker::FieldInfo]| {
            for f in fields {
                field_types.insert((owner, f.name.clone()), checker.defaulted(&f.ty));
            }
        };
        match &info.shape {
            checker::Shape::Record(fields) => add(*def, fields),
            checker::Shape::Union(variants) => {
                for v in variants {
                    add(v.def, &v.fields);
                }
            }
            checker::Shape::Other => {}
        }
    }
    let local_types = checker
        .locals
        .iter()
        .map(|(def, ty)| (*def, checker.defaulted(ty)))
        .collect();
    let drop_types = checker.drop_types.clone();
    let drop_locals = (checker.locals.iter())
        .filter(|(_, t)| checker.holds_drop(&checker.defaulted(t)))
        .map(|(d, _)| *d)
        .collect();
    let mut errors = checker.errors;
    errors.sort_by_key(|e| e.span.start);
    errors.dedup_by_key(|e| e.span.start);
    Checked {
        errors,
        warnings: checker.warnings,
        literal_types,
        big_constants: checker.big_constants,
        float_constants,
        gpu_maps,
        expr_types,
        uses: checker.fn_uses,
        field_types,
        local_types,
        drop_types,
        drop_locals,
        agent_models,
        ask_models,
        agent_settings,
    }
}
