// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

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

/// A `test` block's uses (spec 11.8): it may use what `uses` and `with`
/// give it, and the console, the clock and random numbers, which a test
/// run captures, freezes and seeds.
pub(crate) struct TestNeeds {
    pub(crate) allowed: BTreeSet<String>,
    pub(crate) needs: Vec<NeedAt>,
}

/// What every `test` block may use.
const TEST_ALWAYS: [&str; 3] = ["Console", "Clock", "Random"];

/// A function as this check sees it.
struct FnInfo<'f> {
    decl: &'f FnDecl,
    is_pub: bool,
    /// What its `uses` clause lists; `None` when it has none.
    declared: Option<BTreeSet<String>>,
    /// The agent whose handler it is: then `declared` is the agent's `can`.
    agent: Option<&'f str>,
    /// The interface whose method it is: what its `uses` lists is all a
    /// caller knows, and all an implementation may use (decision 0147).
    iface: Option<&'f str>,
    /// What it may use without listing it: its package's defaults, none
    /// for an agent's handler (spec 8.1, rule 7).
    defaults: BTreeSet<String>,
}

/// Whether `declared` covers `cap`: `Payments` covers `Payments.refund`.
/// The parts of a built-in capability `can` grants alone (spec 10.12).
const NARROW: [&str; 2] = ["Fs.read", "Fs.write"];

fn covers(declared: &BTreeSet<String>, cap: &str) -> bool {
    declared.iter().any(|d| kavach_resolver::cap_covers(d, cap))
}

impl Checker<'_> {
    pub(crate) fn note_cap(&mut self, cap: &str, span: Span) {
        self.check_round_effect(&format!("using `{cap}`"), span);
        self.note(Need::Cap(cap.to_string()), span);
    }

    pub(crate) fn note_fn(&mut self, def: DefId, span: Span) {
        self.note(Need::Fn(def), span);
    }

    fn note(&mut self, need: Need, span: Span) {
        // Each lambda around the code needs it too (spec 3.4).
        for l in &self.lambdas {
            let at = self.lambda_needs.entry((l.start, l.end)).or_default();
            at.push(need.clone());
        }
        if let Some(f) = self.current_fn {
            let in_contract = self.in_contract;
            self.needs.entry(f).or_default().push(NeedAt {
                need,
                span,
                in_contract,
            });
        } else if let Some(t) = self.current_test {
            self.test_needs[t].needs.push(NeedAt {
                need,
                span,
                in_contract: false,
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
                       agent: Option<&'f str>,
                       iface: Option<&'f str>| {
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
                    None if f.uses.is_empty() && iface.is_none() => None,
                    None => Some(listed),
                };
                out.insert(
                    def,
                    FnInfo {
                        decl: f,
                        is_pub,
                        declared,
                        agent,
                        iface,
                        defaults: match agent {
                            Some(_) => BTreeSet::new(),
                            None => c.defaults_at(f.name.span.start),
                        },
                    },
                );
            }
        };
        for item in &file.items {
            let methods = match &item.kind {
                ItemKind::Fn(f) => {
                    add(self, f, item.is_pub, None, None, None);
                    continue;
                }
                ItemKind::Interface(i) => {
                    for m in &i.methods {
                        if let ItemKind::Fn(f) = &m.kind {
                            add(self, f, true, None, None, Some(&i.name.name));
                        }
                    }
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
                            add(self, f, true, Some(BTreeSet::from([own])), None, None);
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
                            add(self, f, true, Some(can.clone()), Some(&a.name.name), None);
                        }
                    }
                    continue;
                }
                _ => continue,
            };
            for m in methods {
                if let ItemKind::Fn(f) = &m.kind {
                    add(self, f, m.is_pub, None, None, None);
                }
            }
        }
        out
    }

    /// Infers `uses` and reports every capability used without permission.
    pub(crate) fn check_capabilities(&mut self, file: &File) {
        let infos = self.fn_infos(file);
        // An interface's method may use its package's defaults (0148).
        let mut effective: HashMap<DefId, BTreeSet<String>> = (infos.iter())
            .map(|(d, i)| {
                let mut caps = i.declared.clone().unwrap_or_default();
                if i.iface.is_some() {
                    caps.extend(i.defaults.iter().cloned());
                }
                (*d, caps)
            })
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
        // Inferred functions, and the defaults a function with `uses`
        // uses besides what it lists: grow until nothing changes.
        loop {
            let mut changed = false;
            for (d, info) in &infos {
                if info.iface.is_some() || info.decl.name.name == "main" {
                    continue;
                }
                let mut n = needed(*d, &effective);
                if let Some(listed) = &info.declared {
                    n.retain(|c| covers(&info.defaults, c));
                    n.extend(listed.iter().cloned());
                }
                if effective[d] != n {
                    effective.insert(*d, n);
                    changed = true;
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
        // Module constants need no capability, but a script's (spec 1.13).
        let caps = |d: DefId| needed(d, &effective).first().cloned();
        self.settle_module_consts(file, &caps);
        let mut errors = Vec::new();
        for (d, info) in &infos {
            let name = &info.decl.name.name;
            if name == "main" {
                continue;
            }
            let checks_uses = info.declared.is_some() || info.is_pub;
            let mut allowed = info.declared.clone().unwrap_or_default();
            allowed.extend(info.defaults.iter().cloned());
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
                    // Direct and written uses are reported where the call is
                    // checked; here only uses a function doesn't write.
                    let unwritten = matches!(&n.need, Need::Fn(f) if infos.get(f).is_some_and(|i| i.decl.uses.is_empty()));
                    let iface = match &n.need {
                        Need::Fn(f) => infos.get(f).and_then(|i| i.iface),
                        Need::Cap(_) => None,
                    };
                    match (unwritten, what, caps.iter().next(), iface) {
                        (true, Some(callee), Some(cap), Some(i)) => errors.push((n.span, format!("a contract can't call `{callee}`: an implementation of `{i}.{callee}` may use {cap}, a default capability of its package, and contracts must be pure (spec 7.5, 8.1)"))),
                        (true, Some(callee), Some(cap), None) => errors.push((n.span, format!("a contract can't call `{callee}`: it uses {cap}, and contracts must be pure (spec 7.5)"))),
                        _ => {}
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
                // `can = ["Fs"]` covers `Fs.read`; `["Fs.read"]` covers only
                // it; `System` doesn't cover `System.critical.read` (8.1).
                let base = cap.split_once('.').map_or(cap.as_str(), |(c, _)| c);
                let apart =
                    (kavach_resolver::APART.iter()).any(|a| kavach_resolver::cap_covers(a, cap));
                // A part of a program's own capability counts as all of it.
                let shown = if NARROW.contains(&cap.as_str()) || apart {
                    cap.as_str()
                } else {
                    base
                };
                if granted
                    .iter()
                    .any(|g| kavach_resolver::cap_covers(g, cap) || g == shown)
                    || !reported.insert((package, shown.to_string()))
                {
                    continue;
                }
                errors.push((
                    info.decl.name.span,
                    format!(
                        "`{}` in the package `{package}` uses {shown}, which the package isn't granted: add \"{shown}\" to `can` in its entry in kavach.toml, and in the entry of each package it comes through, if you trust them (spec 10.12)",
                        info.decl.name.name
                    ),
                ));
            }
        }
        // A `test` block runs with no capabilities unless given some.
        for t in std::mem::take(&mut self.test_needs) {
            let mut reported = BTreeSet::new();
            for n in &t.needs {
                let caps = match &n.need {
                    Need::Cap(c) => BTreeSet::from([c.clone()]),
                    Need::Fn(f) => effective.get(f).cloned().unwrap_or_default(),
                };
                for cap in caps {
                    let base = cap.split_once('.').map_or(cap.as_str(), |(c, _)| c);
                    if TEST_ALWAYS.contains(&base)
                        || covers(&t.allowed, &cap)
                        || !reported.insert(cap.clone())
                    {
                        continue;
                    }
                    errors.push((n.span, format!("this test uses {cap}, but a test runs with no capabilities unless given some: write `uses {base}` after its name, or give it a fake with `with` (spec 11.8)")));
                }
            }
        }
        // An implementation uses only what its interface's method lists:
        // its `uses` and its package's defaults, `effective` above.
        let allowed = (infos.iter())
            .filter(|(_, i)| i.iface.is_some())
            .map(|(d, _)| (*d, effective[d].clone()))
            .collect();
        errors.extend(self.implementations_overreach(&effective, &allowed));
        let lambdas = std::mem::take(&mut self.lambda_needs);
        errors.extend(self.limits_broken(&effective, &lambdas));
        errors.sort_by_key(|(s, _)| s.start);
        for (span, text) in errors {
            self.error(span, text);
        }
    }
}
