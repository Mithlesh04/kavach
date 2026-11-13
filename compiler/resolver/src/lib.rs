//! The Kavach resolver: links every name in a file to its declaration
//! (spec 1.4, 1.5, 10.1) and checks the rules that only need names:
//! unknown names, duplicates, naming styles, and assignments to `let`
//! bindings and parameters.
//!
//! Field and method names (`order.total`) need types, so the type checker
//! resolves those.

mod decls;
mod modules;
mod naming;
mod prelude;
mod resolver;
mod unused;
mod walk;

use kavach_ast::{File, Span};
pub use modules::Module;
pub use prelude::CAPABILITIES;
use std::collections::HashMap;

/// Index into [`Resolution::defs`].
pub type DefId = usize;

#[derive(Clone, Debug, PartialEq)]
pub struct Def {
    pub name: String,
    pub kind: DefKind,
    /// Where it is declared; `None` for built-ins.
    pub span: Option<Span>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefKind {
    Fn,
    Type,
    /// A union variant, with the union it belongs to.
    Variant {
        of: DefId,
    },
    Interface,
    Capability,
    Agent,
    /// A name brought in by `use`.
    Import,
    GenericParam,
    Param,
    Local {
        mutable: bool,
    },
    /// A field name inside a record's `invariant`.
    Field,
    /// `result` inside `ensure` (spec 7.2).
    Result,
    /// Part of the prelude (spec 10.2) or a built-in capability (spec 8.1).
    Builtin,
    /// Another module, brought in with `use orders` (spec 10.2).
    Module,
    /// A tag for literals, `literal sql: Query check …` (spec 10.15).
    Literal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
}

#[derive(Clone, Debug, Default)]
pub struct Resolution {
    pub defs: Vec<Def>,
    /// Every resolved name use, keyed by the start offset of its token.
    pub uses: HashMap<u32, DefId>,
    /// Bare variant names found in several unions (`Open` in `Door` and
    /// `Shop`). The type checker picks one from context (spec 2.6).
    pub ambiguous: HashMap<u32, Vec<DefId>>,
    pub errors: Vec<Diagnostic>,
    pub warnings: Vec<Diagnostic>,
    /// The module each `DefKind::Module` definition names, by index into
    /// the modules given to [`resolve_modules`].
    pub module_defs: HashMap<DefId, usize>,
    /// The modules, in order; the first is the entry file.
    pub modules: Vec<Module>,
    /// Each declared name in the wrong style (spec 1.5), by the span of
    /// the declaration's name, with its right spelling: `kavach fmt`
    /// renames it and its uses.
    pub name_fixes: Vec<(Span, String)>,
}

impl Resolution {
    /// The module the source at `offset` belongs to.
    pub fn module_at(&self, offset: u32) -> usize {
        self.modules
            .iter()
            .position(|m| offset >= m.start && offset < m.end)
            .unwrap_or(0)
    }
}

impl Resolution {
    /// The type `name` declared by the standard library module `module`
    /// that every program has, such as `Json` in `std.json` (spec 2.12).
    pub fn builtin_type(&self, module: &str, name: &str) -> Option<DefId> {
        let m = self.modules.iter().find(|m| m.name == module)?;
        self.defs.iter().position(|d| {
            d.name == name
                && d.kind == DefKind::Type
                && d.span
                    .is_some_and(|s| s.start >= m.start && s.start < m.end)
        })
    }

    /// A type declared by any standard library module written in Kavach
    /// (`Json`, `FsError`).
    pub fn std_type(&self, name: &str) -> Option<DefId> {
        self.modules
            .iter()
            .filter(|m| m.name.starts_with("std."))
            .find_map(|m| self.builtin_type(&m.name, name))
    }

    /// What the name starting at `offset` refers to.
    pub fn def_at(&self, offset: u32) -> Option<&Def> {
        self.uses.get(&offset).map(|&id| &self.defs[id])
    }
}

/// Resolves all names in one file.
pub fn resolve(file: &File) -> Resolution {
    let whole = Module {
        name: "main".into(),
        start: 0,
        end: u32::MAX,
        package: String::new(),
    };
    resolve_modules(file, &[whole])
}

/// Resolves the files of a project, parsed together: `modules` says where
/// each one is. The first module is the entry file (spec 10.3).
pub fn resolve_modules(file: &File, modules: &[Module]) -> Resolution {
    resolver::Resolver::new().run_modules(file, modules)
}
