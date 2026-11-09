//! Code that never runs (spec 4.7): a `match` arm that can't match, an
//! `else if` that can't be true, and a `while` loop that never ends. Each
//! is a warning that says what to change. The checks look only at what
//! the code says (names, numbers, patterns), so they never warn about
//! code that can run.

use crate::checker::Checker;
use kavach_ast::visit::{VisitMut, walk_expr, walk_file, walk_stmt};
use kavach_ast::*;
use kavach_resolver::Resolution;
use std::collections::HashSet;

impl Checker<'_> {
    pub(crate) fn check_never_runs(&mut self, file: &File) {
        let mut inout = HashSet::new();
        for item in &file.items {
            inout_fns(item, &mut inout);
        }
        let mut found = NeverRuns {
            res: self.res,
            inout,
            found: Vec::new(),
            seen: HashSet::new(),
        };
        walk_file(&mut found, &mut file.clone());
        for (span, message) in found.found {
            self.warn(span, &message);
        }
    }
}

/// The names of functions and methods with an `inout` parameter: a call
/// to one may change a variable passed to it.
fn inout_fns(item: &Item, out: &mut HashSet<String>) {
    match &item.kind {
        ItemKind::Fn(f) if f.params.iter().any(|p| p.is_inout) => {
            out.insert(f.name.name.clone());
        }
        ItemKind::Type(t) => {
            if let TypeBody::Record(r) = &t.body {
                r.methods.iter().for_each(|m| inout_fns(m, out));
            }
        }
        ItemKind::Extend(e) => e.methods.iter().for_each(|m| inout_fns(m, out)),
        _ => {}
    }
}

struct NeverRuns<'r> {
    res: &'r Resolution,
    inout: HashSet<String>,
    found: Vec<(Span, String)>,
    /// Conditions already reported, as a chain's later `if`s are visited
    /// again on their own.
    seen: HashSet<u32>,
}

impl VisitMut for NeverRuns<'_> {
    fn expr(&mut self, e: &mut Expr) {
        match &e.kind {
            ExprKind::Match { arms, .. } => self.arms(arms),
            ExprKind::If { .. } => self.chain(e),
            _ => {}
        }
        walk_expr(self, e);
    }

    fn stmt(&mut self, s: &mut Stmt) {
        match &s.kind {
            StmtKind::While { cond, body, .. } => self.endless(cond, body),
            StmtKind::For {
                pattern,
                iter,
                body,
                ..
            } => self.past_end(pattern, iter, body),
            _ => {}
        }
        walk_stmt(self, s);
    }
}

/// What an unguarded arm's pattern matches, to compare arms: everything,
/// every value of a variant, or one value.
#[derive(PartialEq)]
enum Covers {
    All,
    Variant(String),
    Value(String),
}

fn covers(res: &Resolution, p: &Pattern) -> Option<Covers> {
    let catch_all = |p: &Pattern| matches!(p.kind, PatternKind::Wildcard | PatternKind::Binding(_));
    Some(match &p.kind {
        PatternKind::Wildcard | PatternKind::Binding(_) => Covers::All,
        PatternKind::Variant { fields, .. } => {
            let name = variant(res, p)?;
            let whole = fields
                .as_ref()
                .is_none_or(|fs| fs.iter().all(|f| catch_all(&f.pattern)));
            if !whole {
                return None;
            }
            Covers::Variant(name)
        }
        PatternKind::Literal(e) => Covers::Value(literal(e)?),
        PatternKind::None => Covers::Value("none".into()),
        _ => return None,
    })
}

/// A literal's value, as it is written.
fn literal(e: &Expr) -> Option<String> {
    Some(match &e.kind {
        ExprKind::Int(v) => v.to_string(),
        // Text without `{…}` in it.
        ExprKind::Str(parts) => match &parts[..] {
            [] => "\"\"".to_string(),
            [StrPart::Text(s)] => format!("{s:?}"),
            _ => return None,
        },
        ExprKind::Bool(b) => b.to_string(),
        ExprKind::Char(c) => format!("{c:?}"),
        ExprKind::Unary(UnaryOp::Neg, x) => format!("-{}", literal(x)?),
        _ => return None,
    })
}

/// The variant a pattern names, if it is one: the definition it resolves
/// to, or else its path as written (`FsError.Denied` is not
/// `InboxError.Denied`).
fn variant(res: &Resolution, p: &Pattern) -> Option<String> {
    let PatternKind::Variant { path, .. } = &p.kind else {
        return None;
    };
    let last = path.segments.last()?;
    Some(match res.uses.get(&last.span.start) {
        Some(d) => format!("#{d}"),
        None => path
            .segments
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join("."),
    })
}

impl NeverRuns<'_> {
    fn arms(&mut self, arms: &[MatchArm]) {
        let mut before: Vec<Covers> = Vec::new();
        for arm in arms {
            let hidden = before.iter().find_map(|b| match b {
                Covers::All => Some("the arm above it matches every value first".to_string()),
                Covers::Variant(v) if variant(self.res, &arm.pattern).as_ref() == Some(v) => {
                    Some("an arm above it matches every value of that variant first".to_string())
                }
                b if covers(self.res, &arm.pattern).as_ref() == Some(b) => {
                    Some("an arm above it has the same pattern".to_string())
                }
                _ => None,
            });
            if let Some(why) = hidden {
                self.found.push((
                    arm.pattern.span,
                    format!("this arm never matches: {why}. Remove it, or move it up (spec 4.7)"),
                ));
            }
            if arm.guard.is_none() {
                before.extend(covers(self.res, &arm.pattern));
            }
        }
    }
}

mod chains;
mod loops;
