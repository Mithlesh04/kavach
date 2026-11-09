//! Semantic tokens (to-do item 5): marks the grammar alone can't give, so
//! a reader sees at once where a name comes from. A module brought in with
//! `use`, and every name used through it or brought in from one, is
//! `imported`; a call to a built-in method (`xs.push(1)`) is a method that
//! is `defaultLibrary`, and a method the program defines is a plain one.
//! Everything else keeps the grammar's colors.

use crate::position::position;
use kavach_ast::visit::{VisitMut, walk_expr, walk_file};
use kavach_ast::{Expr, ExprKind, Ident, ItemKind, TypeBody, TypeKind};
use kavach_checker::Ty;
use kavach_driver::Compiled;
use kavach_resolver::DefKind;
use serde_json::{Value, json};
use std::collections::HashSet;

/// The token types and modifiers, in the order the legend lists them.
pub const TYPES: [&str; 5] = ["namespace", "function", "method", "variable", "type"];
pub const MODIFIERS: [&str; 2] = ["imported", "defaultLibrary"];

const NAMESPACE: u32 = 0;
const FUNCTION: u32 = 1;
const METHOD: u32 = 2;
const VARIABLE: u32 = 3;
const TYPE: u32 = 4;
const IMPORTED: u32 = 1;
const LIBRARY: u32 = 2;

/// The legend sent with the server's capabilities.
pub fn legend() -> Value {
    json!({"tokenTypes": TYPES, "tokenModifiers": MODIFIERS})
}

/// One mark: line (from 0), column and length in UTF-16 code units, as
/// the protocol counts them, its type (in `TYPES`) and modifiers (bits of
/// `MODIFIERS`).
pub type Mark = (u64, u64, u64, u32, u32);

/// `textDocument/semanticTokens/full` for the file at `path` (its text
/// `text`): the marks, as the protocol's relative numbers.
pub fn tokens(c: &Compiled, path: &str, text: &str) -> Value {
    let (mut line, mut col) = (0u64, 0u64);
    let mut data = Vec::new();
    for (l, ch, len, ty, modifiers) in marks(c, path, text) {
        let delta_col = if l == line { ch - col } else { ch };
        data.extend([
            l - line,
            delta_col,
            len,
            u64::from(ty),
            u64::from(modifiers),
        ]);
        (line, col) = (l, ch);
    }
    json!({"data": data})
}

/// The marks of the file at `path` (its text `text`), in order.
pub fn marks(c: &Compiled, path: &str, text: &str) -> Vec<Mark> {
    let Some(file) = c
        .files
        .iter()
        .find(|f| crate::navigate::same_file(&f.path, path))
    else {
        return Vec::new();
    };
    let end = c
        .files
        .iter()
        .find(|f| f.start > file.start)
        .map_or(u32::MAX, |f| f.start);
    let mut marks = Marks {
        c,
        methods: own_methods(c),
        modules: modules(c),
        found: Vec::new(),
    };
    let mut ast = c.file.clone();
    walk_file(&mut marks, &mut ast);
    let mut found: Vec<(u32, u32, u32, u32)> = marks
        .found
        .into_iter()
        .filter(|(at, ..)| *at >= file.start && *at < end)
        .collect();
    found.sort_unstable();
    found.dedup_by_key(|t| t.0);
    found
        .into_iter()
        .map(|(at, len, ty, modifiers)| {
            let start = position(text, (at - file.start) as usize);
            let line = start["line"].as_u64().unwrap_or(0);
            let col = start["character"].as_u64().unwrap_or(0);
            (line, col, u64::from(len), ty, modifiers)
        })
        .collect()
}

/// `kavach query tokens`: the marks of the file at `path`, as one JSON
/// line, `{"file": …, "tokens": [[line, col, len, type, [modifiers]], …]}`,
/// or `{"file": …, "error": …}` when it doesn't compile. In a project, its
/// local modules come with it, as for `kavach check`.
pub fn json_line(path: &str) -> String {
    // The same on every system, so a build reads it the same way.
    let file = path.replace('\\', "/");
    let line = match std::fs::read_to_string(path) {
        Err(e) => json!({"file": file, "error": format!("can't read it: {e}")}),
        Ok(text) => {
            let (c, _) = crate::diagnostics::check(path, &text, |_| None);
            let first = c
                .messages
                .iter()
                .find(|m| m.severity == kavach_driver::Severity::Error);
            match first {
                Some(m) => json!({"file": file, "error": m.text}),
                None => {
                    let tokens: Vec<Value> = marks(&c, path, &text)
                        .into_iter()
                        .map(|(l, col, len, ty, modifiers)| {
                            let names: Vec<&str> = MODIFIERS
                                .iter()
                                .enumerate()
                                .filter(|(i, _)| modifiers & (1 << i) != 0)
                                .map(|(_, m)| *m)
                                .collect();
                            json!([l, col, len, TYPES[ty as usize], names])
                        })
                        .collect();
                    json!({"file": file, "tokens": tokens})
                }
            }
        }
    };
    line.to_string()
}

/// The names `use` gives modules (`use std.math` gives `math`): a `use`
/// with no `{…}` brings in a module, not the names in it.
fn modules(c: &Compiled) -> HashSet<String> {
    c.own_items()
        .iter()
        .filter_map(|item| match &item.kind {
            ItemKind::Use(u) if u.names.is_empty() => u
                .alias
                .as_ref()
                .or(u.path.segments.last())
                .map(|n| n.name.clone()),
            _ => None,
        })
        .collect()
}

/// The methods the program declares, as (type name, method name): on its
/// records, in `extend`, and in its interfaces.
fn own_methods(c: &Compiled) -> HashSet<(String, String)> {
    let mut out = HashSet::new();
    let names = |items: &[kavach_ast::Item]| -> Vec<String> {
        items
            .iter()
            .filter_map(|m| match &m.kind {
                ItemKind::Fn(f) => Some(f.name.name.clone()),
                _ => None,
            })
            .collect()
    };
    for item in c.own_items() {
        let (owner, methods) = match &item.kind {
            ItemKind::Type(t) => match &t.body {
                TypeBody::Record(r) => (t.name.name.clone(), names(&r.methods)),
                _ => continue,
            },
            ItemKind::Interface(i) => (i.name.name.clone(), names(&i.methods)),
            ItemKind::Extend(e) => match &e.target.kind {
                TypeKind::Named { path, .. } => match path.segments.last() {
                    Some(last) => (last.name.clone(), names(&e.methods)),
                    None => continue,
                },
                _ => continue,
            },
            _ => continue,
        };
        out.extend(methods.into_iter().map(|m| (owner.clone(), m)));
    }
    out
}

struct Marks<'c> {
    c: &'c Compiled,
    methods: HashSet<(String, String)>,
    /// The names of the modules brought in with `use` (`modules`).
    modules: HashSet<String>,
    /// (where, length, type, modifiers), in the combined source.
    found: Vec<(u32, u32, u32, u32)>,
}

impl Marks<'_> {
    fn mark(&mut self, id: &Ident, ty: u32, modifiers: u32) {
        self.mark_at(id.span.start, &id.name, ty, modifiers);
    }

    fn mark_at(&mut self, at: u32, name: &str, ty: u32, modifiers: u32) {
        // Counted in UTF-16, as the protocol and JavaScript count them.
        let len = name.encode_utf16().count() as u32;
        self.found.push((at, len, ty, modifiers));
    }

    /// A use of a project module's member, `orders.total`, which the
    /// resolver folds into the name `total` spanning the whole path: marks
    /// the module and the member. Whether it was folded shows in the span.
    fn folded(&mut self, e: &Expr, id: &Ident, called: bool) {
        if e.span.start >= id.span.start {
            return;
        }
        let c = self.c;
        let path = c.src.get(e.span.start as usize..id.span.start as usize);
        let module = path.unwrap_or("").trim_end_matches('.');
        if !module.is_empty() && module.chars().all(|ch| ch.is_alphanumeric() || ch == '_') {
            self.mark_at(e.span.start, module, NAMESPACE, IMPORTED);
            self.member(id, called);
        }
    }

    /// Whether `id` names a module brought in with `use`.
    fn is_module(&self, id: &Ident) -> bool {
        match self.kind(id) {
            Some(DefKind::Module) => true,
            Some(DefKind::Import) => self.modules.contains(&id.name),
            _ => false,
        }
    }

    /// The kind of definition `id` resolves to.
    fn kind(&self, id: &Ident) -> Option<DefKind> {
        let res = &self.c.resolution;
        res.uses.get(&id.span.start).map(|d| res.defs[*d].kind)
    }

    /// A name used through a module: a type when it starts with a capital.
    fn member(&mut self, name: &Ident, called: bool) {
        let ty = match (called, name.name.starts_with(char::is_uppercase)) {
            (true, _) => FUNCTION,
            (false, true) => TYPE,
            (false, false) => VARIABLE,
        };
        self.mark(name, ty, IMPORTED);
    }

    /// The name of a receiver's type, for the program's own methods.
    fn owner(&self, base: &Expr) -> Option<String> {
        let ty = self.c.expr_types.get(&(base.span.start, base.span.end))?;
        Some(match ty {
            Ty::Adt(d, _) => self.c.resolution.defs[*d].name.clone(),
            Ty::Prim(p) => p.name().to_string(),
            Ty::Builtin(n, _) => n.to_string(),
            _ => return None,
        })
    }
}

impl VisitMut for Marks<'_> {
    fn expr(&mut self, e: &mut Expr) {
        match &e.kind {
            ExprKind::Call { callee, .. } => {
                if let ExprKind::Name(id) = &callee.kind {
                    self.folded(callee, id, true);
                }
                if let ExprKind::Field { base, name, .. } = &callee.kind {
                    match &base.kind {
                        ExprKind::Name(m) if self.is_module(m) => {
                            self.mark(m, NAMESPACE, IMPORTED);
                            self.member(name, true);
                        }
                        _ => {
                            let own = self
                                .owner(base)
                                .is_some_and(|o| self.methods.contains(&(o, name.name.clone())));
                            self.mark(name, METHOD, if own { 0 } else { LIBRARY });
                        }
                    }
                }
            }
            ExprKind::Field { base, name, .. } => {
                if let ExprKind::Name(m) = &base.kind
                    && self.is_module(m)
                {
                    self.mark(m, NAMESPACE, IMPORTED);
                    self.member(name, false);
                }
            }
            ExprKind::Name(id) if e.span.start < id.span.start => self.folded(e, id, false),
            ExprKind::Name(id) if self.kind(id) == Some(DefKind::Import) && !self.is_module(id) => {
                let ty = if id.name.starts_with(char::is_uppercase) {
                    TYPE
                } else {
                    FUNCTION
                };
                self.mark(id, ty, IMPORTED);
            }
            _ => {}
        }
        walk_expr(self, e);
    }
}

#[cfg(test)]
mod tests {
    use super::json_line;

    /// A project's module, a name the protocol counts in UTF-16, and a
    /// file that doesn't compile.
    #[test]
    fn query_tokens_gives_one_json_line_per_file() {
        let dir = std::env::temp_dir().join(format!("kavach-tokens-{}", std::process::id()));
        let src = dir.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(dir.join("kavach.toml"), "[package]\nname = \"demo\"\n").unwrap();
        let main = "use orders\n\nfn main() {\n    const नाम = \"a\"\n    print(नाम.len() + orders.total())\n}\n";
        std::fs::write(src.join("main.kav"), main).unwrap();
        std::fs::write(
            src.join("orders.kav"),
            "pub fn total() -> Int {\n    1\n}\n",
        )
        .unwrap();
        std::fs::write(dir.join("broken.kav"), "fn main() {\n    print(nope)\n}\n").unwrap();
        let path = |p: &std::path::Path| p.to_string_lossy().replace('\\', "/");
        let got = json_line(&path(&src.join("main.kav")));
        let broken = json_line(&path(&dir.join("broken.kav")));
        std::fs::remove_dir_all(&dir).unwrap();
        let tokens = r#""tokens":[[4,14,3,"method",["defaultLibrary"]],[4,22,6,"namespace",["imported"]],[4,29,5,"function",["imported"]]]"#;
        assert!(got.ends_with(&format!("{tokens}}}")), "{got}");
        assert!(broken.contains(r#""error":"#), "{broken}");
    }
}
