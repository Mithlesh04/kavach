//! Each agent's `settings` (spec 8.5, decision 0060), worked out when the
//! program is built, as its `model` is: an `AgentSettings(…)` with plain
//! values, a `const` or a function with no parameters that gives one, and
//! `with { … }` on any of them.

use crate::ai_models::text;
use crate::checker::Checker;
use kavach_ast::*;
use kavach_resolver::DefId;
use std::collections::HashMap;

/// An agent's settings, as written: `None` where the default holds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AgentSetup {
    /// `enforce`, `suggest` or `observe`.
    pub mode: Option<String>,
    pub audit: Option<AuditSetup>,
    pub state: Option<bool>,
    pub journal: Option<bool>,
    pub folder: Option<String>,
    /// `Average`, `Largest` or `NotCounted`: how `tokens` counts the
    /// `ask`s under way (spec 8.3).
    pub under_way: Option<String>,
    /// Where the settings are written, for the driver's messages.
    pub span: (u32, u32),
}

/// An agent's audit log (spec 8.6).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AuditSetup {
    pub file: Option<String>,
    /// `JsonLines`: one JSON object per entry.
    pub json: bool,
    pub otlp: bool,
}

const NOT_KNOWN: &str = "an agent's settings are chosen when the program is built: write `AgentSettings(…)` with plain values, or call a function with no parameters that gives one, such as `standard()`, perhaps `with { … }` (spec 8.5)";

impl Checker<'_> {
    /// Each agent's settings, by the agent's name.
    pub(crate) fn agent_settings(&mut self, file: &File) -> HashMap<String, AgentSetup> {
        let known = self.known_values(file);
        let mut out = HashMap::new();
        let mut wrong = Vec::new();
        for item in &file.items {
            let ItemKind::Agent(a) = &item.kind else {
                continue;
            };
            for m in &a.members {
                let AgentMember::Settings(e) = m else {
                    continue;
                };
                match self.setup_of(e, &known, 0) {
                    Some(mut s) => {
                        s.span = (e.span.start, e.span.end);
                        out.insert(a.name.name.clone(), s);
                    }
                    None => wrong.push(e.span),
                }
            }
        }
        for span in wrong {
            self.error(span, NOT_KNOWN);
        }
        out
    }

    fn setup_of(&self, e: &Expr, known: &HashMap<DefId, Expr>, depth: u32) -> Option<AgentSetup> {
        if depth > 8 {
            return None;
        }
        match &e.kind {
            ExprKind::Name(id) => {
                let d = self.res.uses.get(&id.span.start)?;
                self.setup_of(known.get(d)?, known, depth + 1)
            }
            // `standard() with { mode: AgentMode.Observe }`: the fields replaced.
            ExprKind::With { base, fields } => {
                let mut s = self.setup_of(base, known, depth + 1)?;
                for (name, value) in fields {
                    set_field(&mut s, &name.name, value)?;
                }
                Some(s)
            }
            ExprKind::Call { callee, args, .. } => {
                let ExprKind::Name(f) = &callee.kind else {
                    return None;
                };
                if f.name != "AgentSettings" {
                    let d = self.res.uses.get(&f.span.start)?;
                    return match args.is_empty() {
                        true => self.setup_of(known.get(d)?, known, depth + 1),
                        false => None,
                    };
                }
                let mut s = AgentSetup::default();
                for a in args {
                    set_field(&mut s, &a.name.as_ref()?.name, &a.value)?;
                }
                Some(s)
            }
            _ => None,
        }
    }
}

/// The variant `v` names of the union `union`: `AgentMode.Observe` is
/// `Observe` (decision 0070).
fn variant_of<'e>(v: &'e Expr, union: &str) -> Option<&'e str> {
    match &v.kind {
        ExprKind::Field { base, name, .. } => match &base.kind {
            ExprKind::Name(u) if u.name == union => Some(&name.name),
            _ => None,
        },
        _ => None,
    }
}

/// Sets field `name` of `s` to the plain value `v`.
fn set_field(s: &mut AgentSetup, name: &str, v: &Expr) -> Option<()> {
    match (name, &v.kind) {
        ("mode", _) => {
            let mode = variant_of(v, "AgentMode")?;
            ["Enforce", "Suggest", "Observe"]
                .contains(&mode)
                .then_some(())?;
            s.mode = Some(mode.to_lowercase());
        }
        ("state", ExprKind::Bool(b)) => s.state = Some(*b),
        ("journal", ExprKind::Bool(b)) => s.journal = Some(*b),
        ("folder", _) => s.folder = Some(text(v)?),
        ("asks_under_way", _) => {
            let way = variant_of(v, "UnderWay")?;
            ["Average", "Largest", "NotCounted"]
                .contains(&way)
                .then_some(())?;
            s.under_way = Some(way.to_string());
        }
        ("audit", ExprKind::Call { callee, args, .. }) => {
            let ExprKind::Name(f) = &callee.kind else {
                return None;
            };
            (f.name == "Audit").then_some(())?;
            let mut audit = AuditSetup::default();
            for a in args {
                match (a.name.as_ref()?.name.as_str(), &a.value.kind) {
                    ("file", _) => audit.file = Some(text(&a.value)?),
                    ("format", _) => match variant_of(&a.value, "AuditFormat")? {
                        "Text" => audit.json = false,
                        "JsonLines" => audit.json = true,
                        _ => return None,
                    },
                    ("otlp", ExprKind::Bool(b)) => audit.otlp = *b,
                    _ => return None,
                }
            }
            s.audit = Some(audit);
        }
        _ => return None,
    }
    Some(())
}
