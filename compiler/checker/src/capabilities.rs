//! Capabilities checked at compile time (spec 8.1).
//!
//! While bodies are checked, every capability a function uses directly
//! (`print` needs `Console`) and every function it calls is noted. Then:
//!
//! - a private function without `uses` gets what its body needs, worked
//!   out through the functions it calls (rule 3);
//! - a function with `uses` may only use what it lists (rules 1 and 2);
//! - a `pub` function must list everything it uses (rule 3);
//! - `main`, top-level code and `test` blocks may use everything (rule 4);
//! - contracts stay pure, also through functions that infer `uses`;
//! - logging (`std.log`) needs no capability.

use crate::checker::Checker;
use kavach_ast::{AgentMember, File, FnDecl, ItemKind, Span, TypeBody};
use kavach_resolver::DefId;
use std::collections::{BTreeSet, HashMap};

#[derive(Clone, Debug)]
pub(crate) enum Need {
    Cap(String),
    Fn(DefId),
}

#[derive(Clone, Debug)]
pub(crate) struct NeedAt {
    pub(crate) need: Need,
    pub(crate) span: Span,
    pub(crate) in_contract: bool,
}

/// A function as this check sees it.
struct FnInfo<'f> {
    decl: &'f FnDecl,
    is_pub: bool,
    /// What its `uses` clause lists; `None` when it has none.
    declared: Option<BTreeSet<String>>,
    /// The agent whose handler it is: then `declared` is the agent's `can`.
    agent: Option<&'f str>,
}

/// Whether `declared` covers `cap`: `Payments` covers `Payments.refund`.
fn covers(declared: &BTreeSet<String>, cap: &str) -> bool {
    declared.contains(cap)
        || cap
            .split_once('.')
            .is_some_and(|(c, _)| declared.contains(c))
}

impl Checker<'_> {
    pub(crate) fn note_cap(&mut self, cap: &str, span: Span) {
        self.note(Need::Cap(cap.to_string()), span);
    }

    pub(crate) fn note_fn(&mut self, def: DefId, span: Span) {
        self.note(Need::Fn(def), span);
    }

    fn note(&mut self, need: Need, span: Span) {
        if let Some(f) = self.current_fn {
            let in_contract = self.in_contract;
            self.needs.entry(f).or_default().push(NeedAt {
                need,
                span,
                in_contract,
            });
        }
    }

    fn fn_infos<'f>(&self, file: &'f File) -> HashMap<DefId, FnInfo<'f>> {
        let mut out = HashMap::new();
        // `fixed` overrides the `uses` clause: a capability's method uses
        // that method, an agent's handler may use what the agent `can`.
        let mut add = |c: &Self,
                       f: &'f FnDecl,
                       is_pub: bool,
                       fixed: Option<BTreeSet<String>>,
                       agent: Option<&'f str>| {
            if let Some(&def) = c.decl.get(&f.name.span.start) {
                let listed: BTreeSet<String> = f
                    .uses
                    .iter()
                    .map(|p| {
                        p.segments
                            .iter()
                            .map(|s| s.name.as_str())
                            .collect::<Vec<_>>()
                            .join(".")
                    })
                    .collect();
                let declared = match fixed {
                    Some(d) => Some(d),
                    None if f.uses.is_empty() => None,
                    None => Some(listed),
                };
                out.insert(
                    def,
                    FnInfo {
                        decl: f,
                        is_pub,
                        declared,
                        agent,
                    },
                );
            }
        };
        for item in &file.items {
            let methods = match &item.kind {
                ItemKind::Fn(f) => {
                    add(self, f, item.is_pub, None, None);
                    continue;
                }
                ItemKind::Type(t) => match &t.body {
                    TypeBody::Record(r) => &r.methods,
                    _ => continue,
                },
                ItemKind::Extend(e) => &e.methods,
                ItemKind::Capability(c) => {
                    // Granting `Payments` or `Payments.refund` covers it.
                    for m in &c.methods {
                        if let ItemKind::Fn(f) = &m.kind {
                            let own = format!("{}.{}", c.name.name, f.name.name);
                            add(self, f, true, Some(BTreeSet::from([own])), None);
                        }
                    }
                    continue;
                }
                ItemKind::Agent(a) => {
                    // A handler may use only what its agent `can` (spec 8.3).
                    let can: BTreeSet<String> = a
                        .members
                        .iter()
                        .filter_map(|m| match m {
                            AgentMember::Can(paths) => Some(paths),
                            _ => None,
                        })
                        .flatten()
                        .map(|p| {
                            p.segments
                                .iter()
                                .map(|s| s.name.as_str())
                                .collect::<Vec<_>>()
                                .join(".")
                        })
                        .collect();
                    for m in &a.members {
                        if let AgentMember::On(f) = m {
                            add(self, f, true, Some(can.clone()), Some(&a.name.name));
                        }
                    }
                    continue;
                }
                _ => continue,
            };
            for m in methods {
                if let ItemKind::Fn(f) = &m.kind {
                    add(self, f, m.is_pub, None, None);
                }
            }
        }
        out
    }

    /// Infers `uses` and reports every capability used without permission.
    pub(crate) fn check_capabilities(&mut self, file: &File) {
        let infos = self.fn_infos(file);
        let mut effective: HashMap<DefId, BTreeSet<String>> = infos
            .iter()
            .map(|(d, i)| (*d, i.declared.clone().unwrap_or_default()))
            .collect();
        let mut needs = std::mem::take(&mut self.needs);
        // Logging needs no capability (spec 14.12): the program's owner
        // chooses where events go, not the code that writes them.
        needs.retain(|d, _| {
            infos.get(d).is_none_or(|i| {
                let m = self.res.module_at(i.decl.name.span.start);
                self.res.modules[m].name != "std.log"
            })
        });
        let needed = |f: DefId, eff: &HashMap<DefId, BTreeSet<String>>| -> BTreeSet<String> {
            let mut out = BTreeSet::new();
            for n in needs.get(&f).into_iter().flatten() {
                match &n.need {
                    Need::Cap(c) => {
                        out.insert(c.clone());
                    }
                    Need::Fn(d) => out.extend(eff.get(d).cloned().unwrap_or_default()),
                }
            }
            out
        };
        // Inferred functions: grow until nothing changes.
        loop {
            let mut changed = false;
            for (d, info) in &infos {
                if info.declared.is_none() && info.decl.name.name != "main" {
                    let n = needed(*d, &effective);
                    if effective[d] != n {
                        effective.insert(*d, n);
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        // `main` may use any capability, but what it does use is still
        // worth knowing (`kavach query uses`, spec 13.2).
        for (d, info) in &infos {
            if info.declared.is_none() && info.decl.name.name == "main" {
                let n = needed(*d, &effective);
                effective.insert(*d, n);
            }
        }
        self.fn_uses = effective
            .iter()
            .map(|(d, caps)| (*d, caps.iter().cloned().collect()))
            .collect();
        let mut errors = Vec::new();
        for (d, info) in &infos {
            let name = &info.decl.name.name;
            if name == "main" {
                continue;
            }
            let checks_uses = info.declared.is_some() || info.is_pub;
            let allowed = info.declared.clone().unwrap_or_default();
            let mut reported = BTreeSet::new();
            for n in needs.get(d).into_iter().flatten() {
                let (what, caps) = match &n.need {
                    Need::Cap(c) => (None, BTreeSet::from([c.clone()])),
                    Need::Fn(f) => (
                        infos.get(f).map(|i| &i.decl.name.name),
                        effective.get(f).cloned().unwrap_or_default(),
                    ),
                };
                if n.in_contract {
                    // Direct and declared uses are reported where the call is
                    // checked; here only uses inferred through a function.
                    let inferred = matches!(&n.need, Need::Fn(f) if infos.get(f).is_some_and(|i| i.declared.is_none()));
                    if let (true, Some(callee), Some(cap)) = (inferred, what, caps.iter().next()) {
                        errors.push((n.span, format!("a contract can't call `{callee}`: it uses {cap}, and contracts must be pure (spec 7.5)")));
                    }
                    continue;
                }
                if !checks_uses {
                    continue;
                }
                for cap in caps.iter().filter(|c| !covers(&allowed, c)) {
                    if !reported.insert(cap.clone()) {
                        continue;
                    }
                    let via = what.map_or(String::new(), |c| format!(" (through `{c}`)"));
                    let text = if let Some(agent) = info.agent {
                        format!(
                            "the handler `{name}` uses {cap}{via}, but the agent `{agent}` can't: add `{cap}` to its `can` (spec 8.3)"
                        )
                    } else if info.declared.is_some() {
                        format!(
                            "`{name}` uses {cap}{via}, but its `uses` clause doesn't list it: add `{cap}` to `uses` (spec 8.1)"
                        )
                    } else {
                        format!(
                            "`{name}` is `pub`, so it must say what it uses: add `uses {cap}`{via} (spec 8.1)"
                        )
                    };
                    errors.push((n.span, text));
                }
            }
        }
        // A dependency may use only what its entry grants (spec 10.12):
        // each capability is reported once per package.
        let mut reported = BTreeSet::new();
        let mut ordered: Vec<_> = infos.iter().collect();
        ordered.sort_by_key(|(_, i)| i.decl.name.span.start);
        for (d, info) in ordered {
            let module = &self.res.modules[self.res.module_at(info.decl.name.span.start)];
            let package = &module.package;
            let Some((_, granted)) = self.opts.granted.iter().find(|(p, _)| p == package) else {
                continue;
            };
            for cap in effective.get(d).into_iter().flatten() {
                let base = cap.split_once('.').map_or(cap.as_str(), |(c, _)| c);
                if granted.iter().any(|g| g == base) || !reported.insert((package, base)) {
                    continue;
                }
                errors.push((
                    info.decl.name.span,
                    format!(
                        "`{}` in the package `{package}` uses {base}, which the package isn't granted: add \"{base}\" to `can` in its entry in kavach.toml, and in the entry of each package it comes through, if you trust them (spec 10.12)",
                        info.decl.name.name
                    ),
                ));
            }
        }
        errors.sort_by_key(|(s, _)| s.start);
        for (span, text) in errors {
            self.error(span, text);
        }
    }
}
