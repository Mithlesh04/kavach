//! Agents and declared capabilities (spec 8.2, 8.3, 8.8): what `start`
//! takes, and the `Denied` variant every capability method's error type
//! must have, since any capability call can be refused.

use crate::checker::{Checker, Shape};
use crate::ty::{Prim, Ty};
use kavach_ast::{AgentMember, Arg, File, ItemKind, Span, TypeKind};
use kavach_resolver::{DefId, DefKind};
use std::collections::{BTreeMap, BTreeSet};

impl Checker<'_> {
    /// The declared capabilities an agent `can` use, by name: these are what
    /// its `start` takes (built-in ones like `Ai` need no implementation).
    pub(crate) fn collect_agent(&mut self, def: DefId, members: &[AgentMember]) {
        let mut needs = BTreeMap::new();
        for m in members {
            let AgentMember::Can(paths) = m else {
                continue;
            };
            for p in paths {
                let first = &p.segments[0];
                let Some(&cap) = self.res.uses.get(&first.span.start) else {
                    continue;
                };
                if self.res.defs[cap].kind == DefKind::Capability {
                    needs.insert(first.name.clone(), cap);
                }
            }
        }
        self.agent_needs.insert(def, needs);
    }

    /// `Bot.start(Payments: impl, …)`: one implementation, named by its
    /// capability, for every declared capability the agent `can` use.
    pub(crate) fn agent_start(&mut self, agent: DefId, args: &[Arg], span: Span) -> Ty {
        let needs = self.agent_needs.get(&agent).cloned().unwrap_or_default();
        let agent_name = self.res.defs[agent].name.clone();
        let mut given = BTreeSet::new();
        for a in args {
            let t = self.value(&a.value, None);
            let Some(name) = &a.name else {
                self.error(
                    a.value.span,
                    "`start` takes each implementation by its capability's name: `start(Payments: …)`",
                );
                continue;
            };
            let Some(&cap) = needs.get(&name.name) else {
                self.error(
                    name.span,
                    format!("`{agent_name}` can't use a capability `{}`", name.name),
                );
                continue;
            };
            given.insert(name.name.clone());
            let ok = match self.inf.resolve(&t) {
                Ty::Adt(d, _) => self.implements.get(&d).is_some_and(|s| s.contains(&cap)),
                Ty::Error => true,
                _ => false,
            };
            if !ok {
                self.error(
                    a.value.span,
                    format!("this value doesn't implement `{}`", name.name),
                );
            }
        }
        for name in needs.keys().filter(|n| !given.contains(*n)) {
            self.error(
                span,
                format!("`{agent_name}.start` needs an implementation of `{name}`: `{name}: …`"),
            );
        }
        Ty::Adt(agent, Vec::new())
    }

    /// Every capability method's error type is a union with a variant
    /// `Denied(reason: String)` (spec 8.2).
    pub(crate) fn check_capability_errors(&mut self, file: &File) {
        for item in &file.items {
            let ItemKind::Capability(c) = &item.kind else {
                continue;
            };
            for m in &c.methods {
                let ItemKind::Fn(f) = &m.kind else {
                    continue;
                };
                let what = format!("`{}.{}`", c.name.name, f.name.name);
                let Some(errors) = &f.errors else {
                    self.error(
                        f.name.span,
                        format!("{what} can be refused, so it needs an error type with a variant `Denied(reason: String)` (spec 8.2)"),
                    );
                    continue;
                };
                let def = match &errors.kind {
                    TypeKind::Named { path, .. } => path
                        .segments
                        .last()
                        .and_then(|s| self.res.uses.get(&s.span.start).copied()),
                    _ => None,
                };
                let has_denied =
                    def.and_then(|d| self.types.get(&d))
                        .is_some_and(|t| match &t.shape {
                            Shape::Union(vs) => vs.iter().any(|v| {
                                v.name == "Denied"
                                    && v.fields.len() == 1
                                    && v.fields[0].name == "reason"
                                    && v.fields[0].ty == Ty::Prim(Prim::String)
                            }),
                            _ => false,
                        });
                if !has_denied {
                    self.error(
                        errors.span,
                        format!("{what} can be refused, so its error type needs a variant `Denied(reason: String)` (spec 8.2)"),
                    );
                }
            }
        }
    }

    /// `limit` rules and `approve` conditions are checked like any code:
    /// the limited method's parameters have its types (spec 8.3), so in
    /// `amount <= 10_000` the literal is a `Decimal` when `amount` is.
    pub(crate) fn check_agent_rules(&mut self, members: &[AgentMember]) {
        let bool_ty = Ty::Prim(Prim::Bool);
        for m in members {
            match m {
                AgentMember::Limit { target, rules } => {
                    let on_ai = target.segments.first().is_some_and(|s| s.name == "Ai");
                    for r in rules {
                        if !on_ai {
                            self.tokens_only_on_ai(&r.expr);
                        }
                        self.bind_rule_params(target, &r.expr);
                        self.with_ctx("limit", Ty::Unit, None, None, |c| {
                            c.check_expect(&r.expr, &bool_ty);
                        });
                    }
                }
                AgentMember::Approve {
                    target,
                    when,
                    by,
                    within,
                } => {
                    self.bind_rule_params(target, when);
                    let duration = self.builtin_ty("Duration");
                    self.with_ctx("approve", Ty::Unit, None, None, |c| {
                        c.check_expect(when, &bool_ty);
                        c.check_expect(by, &Ty::Prim(Prim::String));
                        if let Some(w) = within {
                            c.check_expect(w, &duration);
                        }
                    });
                }
                AgentMember::Model(e) => {
                    let model = self.builtin_ty("Model");
                    self.with_ctx("model", Ty::Unit, None, None, |c| {
                        c.check_expect(e, &model);
                    });
                }
                AgentMember::Settings(e) => {
                    let settings = self.builtin_ty("AgentSettings");
                    self.with_ctx("settings", Ty::Unit, None, None, |c| {
                        c.check_expect(e, &settings);
                    });
                }
                // An agent can't approve its own requests (spec 14.16).
                AgentMember::Can(paths) => {
                    for p in paths.iter().filter(|p| p.segments[0].name == "Approvals") {
                        self.error(
                            p.segments[0].span,
                            "an agent can't answer approvals: `Approvals` can't be in `can` (8.3)",
                        );
                    }
                }
                _ => {}
            }
        }
    }

    /// `tokens` in a rule on another capability: what `ask`s took counts
    /// only for `Ai` (spec 8.3).
    fn tokens_only_on_ai(&mut self, e: &kavach_ast::Expr) {
        let mut spans = Vec::new();
        kavach_ast::visit::walk_expr(
            &mut TokensIn {
                res: self.res,
                found: &mut spans,
            },
            &mut e.clone(),
        );
        for span in spans {
            self.error(
                span,
                "`tokens` counts what AI calls took: it is for `limit Ai` rules (spec 8.3)",
            );
        }
    }

    /// Gives the parameters a rule names the types of the limited method's
    /// parameters with the same names.
    fn bind_rule_params(&mut self, target: &kavach_ast::Path, e: &kavach_ast::Expr) {
        let (Some(cap), Some(method)) = (
            target
                .segments
                .first()
                .and_then(|s| self.res.uses.get(&s.span.start).copied()),
            target.segments.get(1),
        ) else {
            return;
        };
        let Some(sig) = self
            .methods
            .get(&cap)
            .and_then(|m| m.get(&method.name))
            .cloned()
        else {
            return;
        };
        let used: Vec<(DefId, String)> = self
            .res
            .uses
            .iter()
            .filter(|(start, _)| e.span.start <= **start && **start < e.span.end)
            .map(|(_, d)| *d)
            .filter(|d| {
                self.res.defs[*d].kind == DefKind::Param && self.res.defs[*d].span.is_none()
            })
            .map(|d| (d, self.res.defs[d].name.clone()))
            .collect();
        for (def, name) in used {
            if let Some(p) = sig.params.iter().find(|p| p.name == name) {
                self.locals.insert(def, p.ty.clone());
            }
        }
    }
}

/// The uses of the limit name `tokens` in a rule.
struct TokensIn<'r, 'f> {
    res: &'r kavach_resolver::Resolution,
    found: &'f mut Vec<kavach_ast::Span>,
}

impl kavach_ast::visit::VisitMut for TokensIn<'_, '_> {
    fn expr(&mut self, e: &mut kavach_ast::Expr) {
        if let kavach_ast::ExprKind::Name(id) = &e.kind {
            let builtin = self
                .res
                .uses
                .get(&id.span.start)
                .is_some_and(|d| self.res.defs[*d].kind == kavach_resolver::DefKind::Builtin);
            if builtin && id.name == "tokens" {
                self.found.push(id.span);
            }
        }
        kavach_ast::visit::walk_expr(self, e);
    }
}
