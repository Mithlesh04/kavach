//! Scopes, definitions and lookups.

use crate::naming::{mixed_scripts, pascal_case_fix, snake_case_fix};
use crate::prelude;
use crate::{Def, DefId, DefKind, Diagnostic, Resolution};
use kavach_ast::{Ident, Span};
use std::collections::HashMap;

pub(crate) struct Resolver {
    pub(crate) res: Resolution,
    /// How many `unsafe` blocks the code being resolved is inside: there an
    /// index target may be a `Ptr`, which the checker tells apart.
    pub(crate) in_unsafe: u32,
    /// Innermost last. Scope 0 is the prelude, scope 1 the module.
    scopes: Vec<HashMap<String, DefId>>,
    /// Locals that hide a `let` of the same name in an outer scope.
    pub(crate) hides: HashMap<DefId, DefId>,
    /// Names declared by `const`/`const` later in each open block, to explain
    /// "used before its `const`" instead of just "not defined".
    later: Vec<Vec<String>>,
    /// Every variant name and the variants that have it.
    pub(crate) variants: HashMap<String, Vec<DefId>>,
    /// The variants of each union type, by name.
    pub(crate) union_variants: HashMap<DefId, HashMap<String, DefId>>,
    /// Capability → method → parameter names, for agent `limit` and
    /// `approve` rules, which can use the method's parameters (spec 8.3).
    pub(crate) capability_params: HashMap<String, HashMap<String, Vec<String>>>,
    pub(crate) modules: crate::modules::Modules,
}

impl Resolver {
    pub(crate) fn new() -> Self {
        let mut r = Resolver {
            res: Resolution::default(),
            in_unsafe: 0,
            scopes: vec![HashMap::new()],
            hides: HashMap::new(),
            later: Vec::new(),
            variants: HashMap::new(),
            union_variants: HashMap::new(),
            capability_params: HashMap::new(),
            modules: Default::default(),
        };
        let builtins = prelude::TYPES
            .iter()
            .chain(prelude::CAPABILITIES)
            .chain(prelude::FUNCTIONS);
        for name in builtins {
            let id = r.add_def(name, DefKind::Builtin, None);
            r.scopes[0].insert(name.to_string(), id);
        }
        r.scopes.push(HashMap::new());
        r
    }

    /// Only the prelude, then an empty scope for a module's own names.
    pub(crate) fn reset_scopes(&mut self) {
        self.scopes.truncate(1);
        self.scopes.push(HashMap::new());
    }

    pub(crate) fn take_module_scope(&mut self) -> HashMap<String, DefId> {
        std::mem::take(&mut self.scopes[1])
    }

    pub(crate) fn set_module_scope(&mut self, scope: HashMap<String, DefId>) {
        self.scopes[1] = scope;
    }

    pub(crate) fn finish(mut self) -> Resolution {
        self.res.errors.sort_by_key(|e| e.span.start);
        self.res.warnings.sort_by_key(|e| e.span.start);
        self.res
    }

    pub(crate) fn add_def(&mut self, name: &str, kind: DefKind, span: Option<Span>) -> DefId {
        self.res.defs.push(Def {
            name: name.to_string(),
            kind,
            span,
        });
        self.res.defs.len() - 1
    }

    pub(crate) fn error(&mut self, span: Span, message: impl Into<String>) {
        self.res.errors.push(Diagnostic {
            message: message.into(),
            span,
        });
    }

    pub(crate) fn push(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub(crate) fn pop(&mut self) {
        self.scopes.pop();
    }

    pub(crate) fn push_later(&mut self, names: Vec<String>) {
        self.later.push(names);
    }

    pub(crate) fn pop_later(&mut self) {
        self.later.pop();
    }

    /// Declares `ident` in the innermost scope. Empty names come from
    /// syntax errors and are skipped.
    pub(crate) fn define(&mut self, ident: &Ident, kind: DefKind) -> Option<DefId> {
        if ident.name.is_empty() || ident.name == "_" {
            return None;
        }
        if self.scopes.last().unwrap().contains_key(&ident.name) {
            self.error(
                ident.span,
                format!("`{}` is already declared in this scope", ident.name),
            );
        }
        if let Some((a, b)) = mixed_scripts(&ident.name) {
            self.res.warnings.push(Diagnostic {
                message: format!(
                    "`{}` mixes {a} and {b} letters; look-alike letters can hide mistakes or attacks",
                    ident.name
                ),
                span: ident.span,
            });
        }
        if let Some(names) = self.later.last_mut()
            && let Some(i) = names.iter().position(|n| *n == ident.name)
        {
            names.remove(i);
        }
        // A local hiding one of an outer scope (`let total = total + i`
        // in a loop): the unused warning says so (spec 1.4).
        let outer = (self.scopes.iter().skip(1).rev())
            .find_map(|scope| scope.get(&ident.name).copied())
            .filter(|&o| matches!(self.res.defs[o].kind, DefKind::Local { mutable: true }));
        let id = self.add_def(&ident.name, kind, Some(ident.span));
        if let (DefKind::Local { .. }, Some(o)) = (kind, outer) {
            self.hides.insert(id, o);
        }
        self.scopes
            .last_mut()
            .unwrap()
            .insert(ident.name.clone(), id);
        Some(id)
    }

    /// Adds a name to the innermost scope without duplicate checks
    /// (built-in names inside special scopes).
    pub(crate) fn define_quiet(&mut self, name: &str, kind: DefKind) {
        let id = self.add_def(name, kind, None);
        self.scopes.last_mut().unwrap().insert(name.to_string(), id);
    }

    pub(crate) fn lookup(&self, name: &str) -> Option<DefId> {
        self.scopes.iter().rev().find_map(|s| s.get(name).copied())
    }

    /// Resolves a name used as a value.
    pub(crate) fn use_name(&mut self, ident: &Ident) {
        if ident.name.is_empty() {
            return;
        }
        // A program's own variant named like a built-in type (`List`,
        // `Error`) is what the program means by that name.
        let variants = self.variants.get(&ident.name).map(Vec::as_slice);
        if let Some(id) = self.lookup(&ident.name)
            && (self.res.defs[id].kind != DefKind::Builtin || variants.is_none_or(<[_]>::is_empty))
        {
            self.res.uses.insert(ident.span.start, id);
            return;
        }
        match variants {
            Some([id]) => {
                self.res.uses.insert(ident.span.start, *id);
            }
            Some(ids) if !ids.is_empty() => {
                self.res.ambiguous.insert(ident.span.start, ids.to_vec());
            }
            _ => self.unknown(ident, "is not defined"),
        }
    }

    /// Resolves a name used as a type.
    pub(crate) fn use_type(&mut self, ident: &Ident) {
        if ident.name.is_empty() {
            return;
        }
        match self.lookup(&ident.name) {
            Some(id) => {
                let kind = self.res.defs[id].kind;
                if matches!(
                    kind,
                    DefKind::Fn | DefKind::Local { .. } | DefKind::Param | DefKind::Field
                ) {
                    self.error(
                        ident.span,
                        format!("`{}` is a value, not a type", ident.name),
                    );
                }
                self.res.uses.insert(ident.span.start, id);
            }
            None => self.unknown(ident, "is not a known type"),
        }
    }

    pub(crate) fn unknown(&mut self, ident: &Ident, what: &str) {
        let name = &ident.name;
        let message = if matches!(
            name.as_str(),
            "null" | "nil" | "undefined" | "None" | "NULL"
        ) {
            format!(
                "Kavach has no `{name}`: a value that may be missing has an optional type (`String?`), and the missing value is `none` (spec 2.7)"
            )
        } else if self.later.iter().any(|names| names.contains(name)) {
            format!("`{name}` is used before its `let`: move the declaration above this line")
        } else {
            match self.suggest(name) {
                Some(s) => format!("`{name}` {what}; did you mean `{s}`?"),
                None => format!("`{name}` {what}"),
            }
        };
        self.error(ident.span, message);
    }

    /// The visible name closest to `name`, if it is close enough to be a typo.
    fn suggest(&self, name: &str) -> Option<String> {
        let candidates = self
            .scopes
            .iter()
            .flat_map(|s| s.keys())
            .chain(self.variants.keys());
        candidates
            .map(|c| (edit_distance(name, c), c))
            .filter(|(d, c)| *d <= 2 && *d < name.chars().count().max(2) && c.as_str() != name)
            .min()
            .map(|(_, c)| c.clone())
    }

    /// `what` is the plural shown in the message: "variables", "functions"…
    pub(crate) fn check_snake(&mut self, ident: &Ident, what: &str) {
        if let Some(fixed) = snake_case_fix(&ident.name) {
            self.error(
                ident.span,
                format!("{what} use snake_case: write `{fixed}`"),
            );
            self.res.name_fixes.push((ident.span, fixed));
        }
    }

    pub(crate) fn check_pascal(&mut self, ident: &Ident, what: &str) {
        if let Some(fixed) = pascal_case_fix(&ident.name) {
            self.error(
                ident.span,
                format!("{what} use PascalCase: write `{fixed}`"),
            );
            self.res.name_fixes.push((ident.span, fixed));
        }
    }
}

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = if ca == *cb {
                prev
            } else {
                1 + prev.min(row[j]).min(cur)
            };
            prev = cur;
        }
    }
    row[b.len()]
}
