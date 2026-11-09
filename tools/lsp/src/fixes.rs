//! Quick fixes: for code that never runs (spec 4.7), the `match` arm that
//! can't match, or the `else if` that can't be true, taken out (the code
//! is found in the compile's syntax tree, so the edit takes exactly it);
//! a call to a deprecated function rewritten (spec 12.6, `deprecated.rs`);
//! and a plain `fn` called from a `gpu fn` marked `gpu` (spec 15.1).

use crate::{Document, position};
use kavach_ast::visit::{VisitMut, walk_expr, walk_file};
use kavach_ast::*;
use kavach_driver::{Compiled, Severity};
use serde_json::{Value, json};
use std::collections::HashSet;

/// A fix: its title, and the bytes `from..to` of the file replaced by
/// `text`. A fix that isn't `safe` may be wrong: the editor offers it,
/// `kavach fix` leaves it.
pub(crate) struct Edit {
    pub(crate) title: String,
    pub(crate) from: usize,
    pub(crate) to: usize,
    pub(crate) text: String,
    pub(crate) safe: bool,
}

/// The fix for the message `message` at byte `at` of the file at `path`
/// (its text `text`), if it has one.
pub(crate) fn fix(c: &Compiled, path: &str, text: &str, message: &str, at: usize) -> Option<Edit> {
    let file = c
        .files
        .iter()
        .find(|f| crate::navigate::same_file(&f.path, path))?;
    let base = file.start as usize;
    if message.contains("` is deprecated: ") {
        return crate::deprecated::rewrite(c, base, at);
    }
    if let Some((title, from, to)) = remove_dead(c, base, text, message, at) {
        return Some(Edit {
            title: title.to_string(),
            from,
            to,
            text: String::new(),
            safe: true,
        });
    }
    crate::suggest::suggest(c, base, text, message, at)
}

/// Code that never runs: a title, and the bytes to take out.
fn remove_dead(
    c: &Compiled,
    base: usize,
    text: &str,
    message: &str,
    at: usize,
) -> Option<(&'static str, usize, usize)> {
    let mut find = Find {
        at: (base + at) as u32,
        arm: None,
        branch: None,
    };
    walk_file(&mut find, &mut c.file.clone());
    let local = |x: u32| (x as usize).checked_sub(base);
    if message.starts_with("this arm never matches") {
        let (start, end) = find.arm?;
        let (start, end) = (local(start)?, local(end)?);
        // The whole lines the arm is on, with its comma.
        let from = text[..start].rfind('\n').map_or(0, |n| n + 1);
        let to = text[end..].find('\n').map_or(text.len(), |n| end + n + 1);
        return Some(("Remove this arm", from, to));
    }
    if message.starts_with("this condition is never true here") {
        let (start, end) = find.branch?;
        let (start, end) = (local(start)?, local(end)?);
        // From the `else` before the `if` to the end of its block, and the
        // spaces after it, so a following `else` joins the branch before.
        let mut from = text[..start].rfind("else")?;
        let rest = &text[end..];
        let to = end + (rest.len() - rest.trim_start_matches([' ', '\t']).len());
        // Nothing follows: the spaces before the `else` go too.
        if !text[to..].starts_with("else") {
            from = text[..from].trim_end_matches([' ', '\t']).len();
        }
        return Some(("Remove this branch", from, to));
    }
    None
}

/// The arm whose pattern starts at `at`, and the `else if` whose condition
/// does: their spans in the combined source.
struct Find {
    at: u32,
    arm: Option<(u32, u32)>,
    branch: Option<(u32, u32)>,
}

impl VisitMut for Find {
    fn expr(&mut self, e: &mut Expr) {
        match &e.kind {
            ExprKind::Match { arms, .. } => {
                if let Some(a) = arms.iter().find(|a| a.pattern.span.start == self.at) {
                    self.arm = Some((a.pattern.span.start, a.body.span.end));
                }
            }
            ExprKind::If {
                otherwise: Some(next),
                ..
            } => {
                if let ExprKind::If { cond, then, .. } = &next.kind
                    && cond.span.start == self.at
                {
                    self.branch = Some((next.span.start, then.span.end));
                }
            }
            _ => {}
        }
        walk_expr(self, e);
    }
}

impl crate::Server {
    /// Quick fixes for the diagnostics the editor shows: a plain `fn` that
    /// fits the GPU's rules, called from a `gpu fn`, becomes one (spec
    /// 15.1); code that never runs is taken out (spec 4.7, `fixes.rs`).
    pub(crate) fn code_actions(&self, params: &Value) -> Value {
        let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
        let Some(doc) = self.docs.get(uri) else {
            return json!([]);
        };
        let mut actions = Vec::new();
        for d in params["context"]["diagnostics"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let message = d["message"].as_str().unwrap_or("");
            if let Some(fix) = self.fix_action(uri, doc, d, message) {
                actions.push(fix);
                continue;
            }
            let Some(name) = message
                .split("Mark it `gpu fn ")
                .nth(1)
                .and_then(|rest| rest.split('`').next())
            else {
                continue;
            };
            let Some(at) = plain_fn(&doc.text, name) else {
                continue;
            };
            let here = position::position(&doc.text, at);
            actions.push(json!({
                "title": format!("Mark `{name}` as a `gpu fn`"),
                "kind": "quickfix",
                "diagnostics": [d],
                "isPreferred": true,
                "edit": {"changes": {uri: [{"range": {"start": here, "end": here}, "newText": "gpu "}]}},
            }));
        }
        json!(actions)
    }

    /// The quick fix for a warning with one right answer (`fix`).
    fn fix_action(&self, uri: &str, doc: &Document, d: &Value, message: &str) -> Option<Value> {
        let path = doc.path.as_ref()?;
        let at = position::offset(&doc.text, &d["range"]["start"]);
        let checked = self.checked(path, &doc.text);
        let e = fix(&checked.0, path, &doc.text, message, at)?;
        let range = json!({"start": position::position(&doc.text, e.from), "end": position::position(&doc.text, e.to)});
        Some(json!({
            "title": e.title,
            "kind": "quickfix",
            "diagnostics": [d],
            "isPreferred": e.safe,
            "edit": {"changes": {uri: [{"range": range, "newText": e.text}]}},
        }))
    }
}

/// Where `fn name(` is declared in `text`, not already a `gpu fn`.
fn plain_fn(text: &str, name: &str) -> Option<usize> {
    let wanted = format!("fn {name}(");
    text.match_indices(&wanted).map(|(i, _)| i).find(|&i| {
        let before = text[..i].trim_end_matches([' ', '\t']);
        let line_start = before.is_empty() || before.ends_with('\n');
        line_start || before.ends_with("pub")
    })
}

/// `kavach fix` (spec 10.4) on the file at `path`: the fixes that have one
/// right answer, applied until none is left, and the file written back.
/// A fix that would add an error (a replacement not in scope where the
/// call is) is left out. Gives what it did, one title per fix.
pub fn fix_file(path: &str) -> std::io::Result<Vec<String>> {
    // In a project the other files are read from disk; this one is the
    // text fixed so far.
    let compile = |text: &str| {
        crate::diagnostics::check(path, text, |p| {
            crate::navigate::same_file(p, path).then(|| text.to_string())
        })
        .0
    };
    let errors = |c: &Compiled| {
        (c.messages.iter())
            .filter(|m| m.severity == Severity::Error)
            .count()
    };
    let mut text = std::fs::read_to_string(path)?;
    let mut c = compile(&text);
    let mut done = Vec::new();
    let mut left_out = HashSet::new();
    // Each fix changes the text, so the next is found in a new compile.
    while done.len() < 1000 {
        let Some(file) = c
            .files
            .iter()
            .find(|f| crate::navigate::same_file(&f.path, path))
        else {
            break;
        };
        let start = file.start;
        let next = c.messages.iter().find_map(|m| {
            let at = (m.span.start.checked_sub(start)?) as usize;
            let key = (m.text.clone(), at);
            if at > text.len() || left_out.contains(&key) {
                return None;
            }
            fix(&c, path, &text, &m.text, at)
                .filter(|e| e.safe)
                .map(|e| (key, e))
        });
        let Some((key, e)) = next else {
            break;
        };
        let mut fixed = text.clone();
        fixed.replace_range(e.from..e.to, &e.text);
        let after = compile(&fixed);
        if errors(&after) > errors(&c) {
            left_out.insert(key);
            continue;
        }
        (text, c) = (fixed, after);
        done.push(e.title);
    }
    if !done.is_empty() {
        std::fs::write(path, &text)?;
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::fix;

    /// The text after the fix for the warning on the first `marker` in it.
    fn fixed(src: &str, message: &str, marker: &str) -> String {
        let c = kavach_driver::compile(src, "fix.kav", Default::default());
        let at = src.find(marker).expect("the marker");
        let e = fix(&c, "fix.kav", src, message, at).expect("a fix");
        format!("{}{}{}", &src[..e.from], e.text, &src[e.to..])
    }

    #[test]
    fn an_arm_that_never_matches_is_taken_out() {
        let src = "fn name(n: Int) -> String {\n    match n {\n        _ => \"other\",\n        2 => \"two\",\n    }\n}\n\nfn main() {\n    print(name(2))\n}\n";
        let out = fixed(
            src,
            "this arm never matches: the arm above it matches every value first",
            "2 =>",
        );
        assert_eq!(out, src.replace("        2 => \"two\",\n", ""));
    }

    #[test]
    fn an_else_if_that_is_never_true_is_taken_out() {
        let src = "fn grade(s: Int) -> String {\n    if s > 10 {\n        \"good\"\n    } else if s > 50 {\n        \"great\"\n    } else {\n        \"low\"\n    }\n}\n\nfn main() {\n    print(grade(3))\n}\n";
        let out = fixed(
            src,
            "this condition is never true here: `s > 50` means `s > 10`",
            "s > 50",
        );
        assert_eq!(
            out,
            src.replace("else if s > 50 {\n        \"great\"\n    } ", "")
        );
    }

    /// `kavach fix`: every fix in the file, written back; then nothing.
    #[test]
    fn fix_file_applies_each_fix_once() {
        let path = std::env::temp_dir().join(format!("kavach-fix-{}.kav", std::process::id()));
        let path = path.to_string_lossy().into_owned();
        let src = "fn name(n: Int) -> String {\n    match n {\n        1 => \"one\",\n        1 => \"uno\",\n        _ => \"other\",\n        2 => \"two\",\n    }\n}\n\nfn main() {\n    print(name(2))\n}\n";
        std::fs::write(&path, src).unwrap();
        let done = super::fix_file(&path).unwrap();
        let again = super::fix_file(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(done, ["Remove this arm", "Remove this arm"]);
        assert!(again.is_empty());
        assert_eq!(
            text,
            src.replace("        1 => \"uno\",\n", "")
                .replace("        2 => \"two\",\n", "")
        );
    }

    /// In a project, each fix is found in the text fixed so far, not in
    /// the file on disk: a branch, then an arm, in one file.
    #[test]
    fn fix_file_in_a_project_sees_its_own_fixes() {
        let root = std::env::temp_dir().join(format!("kavach-fix-project-{}", std::process::id()));
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("kavach.toml"), "[package]\nname = \"fix\"\n").unwrap();
        let main = root.join("src").join("main.kav");
        let src = "fn grade(score: Int) -> String {\n    if score > 50 {\n        \"pass\"\n    } else if score > 80 {\n        \"great\"\n    } else {\n        \"try again\"\n    }\n}\n\nfn medal(place: Int) -> String {\n    match place {\n        1 => \"gold\",\n        _ => \"none\",\n        2 => \"silver\",\n    }\n}\n\nfn main() {\n    print(grade(90))\n    print(medal(2))\n}\n";
        std::fs::write(&main, src).unwrap();
        let done = super::fix_file(&main.to_string_lossy()).unwrap();
        let text = std::fs::read_to_string(&main).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(done, ["Remove this branch", "Remove this arm"]);
        assert_eq!(
            text,
            src.replace("else if score > 80 {\n        \"great\"\n    } ", "")
                .replace("        2 => \"silver\",\n", "")
        );
    }

    #[test]
    fn a_last_else_if_leaves_no_space_behind() {
        let src = "fn main() {\n    const x = 5\n    if x > 10 {\n        print(1)\n    } else if x > 20 {\n        print(2)\n    }\n}\n";
        let out = fixed(
            src,
            "this condition is never true here: `x > 20` means `x > 10`",
            "x > 20",
        );
        assert_eq!(
            out,
            src.replace(" else if x > 20 {\n        print(2)\n    }", "")
        );
    }
}
