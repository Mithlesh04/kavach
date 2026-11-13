//! Which functions generated tests call (spec 11.2), and how to make
//! their arguments; the others are listed as not generated, with why.

use super::values::Gen;
use crate::interp::Interp;
use kavach_ast::FnDecl;
use kavach_resolver::DefId;

/// A function to test: its name in reports and found cases, and how to
/// generate its arguments (`self` first, for a method).
pub(super) struct Target<'a> {
    pub(super) name: String,
    pub(super) f: &'a FnDecl,
    pub(super) has_self: bool,
    pub(super) params: Vec<Gen>,
    /// Numbers in its `require`s; values next to them are tried (11.3).
    pub(super) near: Vec<i128>,
}

impl<'a> Interp<'a> {
    pub(super) fn targets(&self) -> (Vec<Target<'a>>, Vec<String>) {
        let (mut targets, mut skipped) = (Vec::new(), Vec::new());
        let mut add = |i: &Self, name: String, f: &'a FnDecl, owner: Option<DefId>| {
            let has_self = f.params.iter().any(|p| p.ty.is_none());
            let decl = owner.and_then(|d| i.records.get(&d).or_else(|| i.unions.get(&d)));
            // `Boxed<S>.sized`: values of a generic type aren't generated.
            let generic_self = has_self && decl.is_some_and(|t| !t.generics.is_empty());
            let owner = owner.map(Gen::Adt);
            let mut params: Vec<Gen> = owner.into_iter().filter(|_| has_self).collect();
            let typed = f.params.iter().filter_map(|p| p.ty.as_ref());
            let gens: Option<Vec<Gen>> = typed.map(|t| i.gen_of(t)).collect();
            // Capabilities are faked (11.5): printing is hidden, declared
            // capabilities and `ask` give generated answers. Files and the
            // network have no fake yet.
            let def = i.decl.get(&f.name.span.start);
            let uses = def
                .and_then(|d| i.c.uses.get(d))
                .cloned()
                .unwrap_or_default();
            let written = f.uses.iter().map(|u| i.text(u.span).to_string());
            let real = uses.into_iter().chain(written).find(|c| !i.faked(c));
            let why = match gens {
                _ if f.c_library.is_some() => {
                    Some("a C function, which runs only in native code (10.7)".to_string())
                }
                _ if !f.generics.is_empty() => Some("generic".to_string()),
                _ if generic_self => Some("its type is generic".to_string()),
                _ if real.is_some() => Some(format!(
                    "uses {} (no fake for it yet)",
                    real.unwrap_or_default()
                )),
                Some(g) => {
                    params.extend(g);
                    None
                }
                None => Some(String::new()),
            };
            // A record holding a `Shared`, as the receiver or a parameter.
            let cant = String::from("takes values that can't be generated yet");
            let why = match why {
                Some(w) if w.is_empty() => Some(cant),
                None if !params.iter().all(|g| i.generable(g, &mut Vec::new())) => Some(cant),
                other => other,
            };
            match why {
                Some(why) => skipped.push(format!("{name}: {why}")),
                None => targets.push(Target {
                    near: numbers_in(f.requires.iter().map(|r| i.text(r.span))),
                    name,
                    f,
                    has_self,
                    params,
                }),
            }
        };
        let mut fns: Vec<&'a FnDecl> = self.fns.values().copied().collect();
        fns.sort_by_key(|f| f.name.span.start);
        // The program's own functions, not the standard library's.
        let own = |f: &&FnDecl| self.c.is_own(f.name.span.start);
        for f in fns
            .into_iter()
            .filter(|f| f.name.name != "main")
            .filter(own)
        {
            add(self, f.name.name.clone(), f, None);
        }
        let mut types: Vec<_> = self.methods.iter().collect();
        types.sort_by_key(|(d, _)| **d);
        for (def, methods) in types {
            let type_name = &self.c.resolution.defs[*def].name;
            let mut ms: Vec<&'a FnDecl> = methods.values().copied().collect();
            ms.sort_by_key(|f| f.name.span.start);
            for f in ms.into_iter().filter(own) {
                add(self, format!("{type_name}.{}", f.name.name), f, Some(*def));
            }
        }
        (targets, skipped)
    }

    /// Whether generated tests have a fake for capability `c` (11.5): the
    /// console (its output is hidden), `Ai`, files, the network, the
    /// clock and the environment (`fakes.rs`), `Random` (nothing to fake yet), and the
    /// program's own.
    pub(super) fn faked(&self, c: &str) -> bool {
        let name = c.split(['.', '(']).next().unwrap_or(c).trim();
        matches!(
            name,
            "Console" | "Ai" | "Fs" | "Net" | "Clock" | "Random" | "Clipboard" | "Env"
        ) || self
            .c
            .resolution
            .defs
            .iter()
            .any(|d| d.name == name && d.kind == kavach_resolver::DefKind::Capability)
    }
}

/// Whole numbers written in `texts` (the `require`s of a function).
fn numbers_in<'t>(texts: impl Iterator<Item = &'t str>) -> Vec<i128> {
    let mut out = Vec::new();
    for text in texts {
        for word in text.split(|c: char| !(c.is_ascii_digit() || c == '_')) {
            if let Ok(n) = word.replace('_', "").parse::<i128>() {
                out.push(n);
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}
