// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Checks file by file while typing (spec 13.12). When an edit leaves a
//! file's declarations as they were, only what is inside its bodies
//! changed, so the other files' messages can't change: the file is
//! checked against the other files' declarations, with their bodies
//! left out. A body stays when it could change what the file is told:
//! it names something the file declares (so the file's names are still
//! used), or the file reaches it through calls (so the capabilities a
//! function uses, worked out from its body, stay right). Of what is left
//! out only the line breaks stay, so every declaration after it keeps
//! its line and its column, where go to definition finds it.

use crate::diagnostics::project_main;
use kavach_ast::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// What a quick check needs of a file, found once for each text.
pub(crate) struct Outline {
    /// The file with each body written `{}`: its declarations. An edit
    /// that keeps it changes no other file's messages.
    pub(crate) skeleton: String,
    /// The names it declares: functions, methods, types, variants,
    /// constants and the like.
    declared: HashSet<String>,
    /// The words of its code, less its declarations' own names.
    words: HashSet<String>,
    bodies: Vec<Body>,
}

/// A body that may be left out: a function's, a method's or a test's.
struct Body {
    /// The block, braces included.
    from: usize,
    to: usize,
    /// The function or method; `None` for a test.
    name: Option<String>,
    /// It must give a value, so it is left out as `{panic("")}`.
    returns: bool,
    words: HashSet<String>,
}

/// Outlines by path, each with the text it was found in.
#[derive(Default)]
pub(crate) struct Outlines(HashMap<String, (String, Outline)>);

impl Outlines {
    /// The outline of the file at `path` whose text is `text`.
    pub(crate) fn of(&mut self, path: &str, text: &str) -> &Outline {
        let fresh = self.0.get(path).is_none_or(|(t, _)| t != text);
        if fresh {
            self.0
                .insert(path.to_string(), (text.to_string(), outline(text)));
        }
        &self.0[path].1
    }

    /// The outline found last for the file at `path`.
    pub(crate) fn get(&self, path: &str) -> Option<&Outline> {
        self.0.get(path).map(|(_, o)| o)
    }
}

/// The outline of one file's text.
pub(crate) fn outline(text: &str) -> Outline {
    let file = kavach_parser::parse(text).file;
    let mut out = Outline {
        skeleton: String::new(),
        declared: HashSet::new(),
        words: HashSet::new(),
        bodies: Vec::new(),
    };
    // Where each declared name is written: not a use of it.
    let mut names = HashSet::new();
    let mut declare = |out: &mut Outline, id: &Ident| {
        out.declared.insert(id.name.clone());
        names.insert(id.span.start as usize);
    };
    for item in &file.items {
        match &item.kind {
            ItemKind::Fn(f) => {
                declare(&mut out, &f.name);
                body_of(text, f, &mut out.bodies);
            }
            ItemKind::Type(t) => {
                declare(&mut out, &t.name);
                match &t.body {
                    TypeBody::Record(r) => {
                        for m in &r.methods {
                            if let ItemKind::Fn(f) = &m.kind {
                                declare(&mut out, &f.name);
                                body_of(text, f, &mut out.bodies);
                            }
                        }
                    }
                    TypeBody::Union(vs) => vs.iter().for_each(|v| declare(&mut out, &v.name)),
                    TypeBody::Alias(_) => {}
                }
            }
            ItemKind::Extend(e) => {
                for m in &e.methods {
                    if let ItemKind::Fn(f) = &m.kind {
                        declare(&mut out, &f.name);
                        body_of(text, f, &mut out.bodies);
                    }
                }
            }
            ItemKind::Interface(i) => declare(&mut out, &i.name),
            ItemKind::Capability(c) => declare(&mut out, &c.name),
            ItemKind::Agent(a) => declare(&mut out, &a.name),
            ItemKind::Literal(l) => declare(&mut out, &l.name),
            ItemKind::Test(t) => push_body(text, &t.body, None, false, &mut out.bodies),
            // A constant's name, and whatever else its line holds.
            ItemKind::Stmt(_) => {
                let span = item.span.start as usize..item.span.end as usize;
                out.declared
                    .extend(words(text.get(span).unwrap_or("")).map(|(_, w)| w.to_string()));
            }
            ItemKind::Use(_) => {}
        }
    }
    out.words = words(text)
        .filter(|(at, _)| !names.contains(at))
        .map(|(_, w)| w.to_string())
        .collect();
    let mut skeleton = String::with_capacity(text.len());
    let mut at = 0;
    for b in &out.bodies {
        skeleton.push_str(&text[at..b.from]);
        skeleton.push_str("{}");
        at = b.to;
    }
    skeleton.push_str(&text[at..]);
    out.skeleton = skeleton;
    out
}

fn body_of(text: &str, f: &FnDecl, into: &mut Vec<Body>) {
    if let Some(body) = &f.body {
        let name = Some(f.name.name.clone());
        push_body(text, body, name, f.ret.is_some(), into);
    }
}

/// The block `b`, if it is one: its text starts with `{` and ends with `}`.
fn push_body(text: &str, b: &Block, name: Option<String>, returns: bool, into: &mut Vec<Body>) {
    let (from, to) = (b.span.start as usize, b.span.end as usize);
    let Some(code) = text.get(from..to) else {
        return;
    };
    if !(code.starts_with('{') && code.ends_with('}')) || into.last().is_some_and(|l| l.to > from) {
        return;
    }
    into.push(Body {
        from,
        to,
        name,
        returns,
        words: words(code).map(|(_, w)| w.to_string()).collect(),
    });
}

/// Each word of `text` (letters, digits and `_`, not starting with a
/// digit), with where it starts. Words in strings and comments count too,
/// as an interpolation in a string is code: a body kept for nothing costs
/// only time.
fn words(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let b = text.as_bytes();
    let mut i = 0;
    std::iter::from_fn(move || {
        let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
        while i < b.len() {
            if word(b[i]) && !(i > 0 && word(b[i - 1])) && !b[i].is_ascii_digit() {
                let start = i;
                while i < b.len() && word(b[i]) {
                    i += 1;
                }
                return Some((start, &text[start..i]));
            }
            i += 1;
        }
        None
    })
}

/// The texts of the files `others`, each with the bodies the file
/// `own` can't be told anything by left out (the module docs say which
/// stay). `module` is the last part of `own`'s module name.
pub(crate) fn left_out(own: &Outline, module: &str, others: &[(&str, &Outline)]) -> Vec<String> {
    let names_own = |words: &HashSet<String>| {
        words.contains(module) || words.iter().any(|w| own.declared.contains(w))
    };
    let mut keep: Vec<Vec<bool>> = others
        .iter()
        .map(|(_, o)| o.bodies.iter().map(|b| names_own(&b.words)).collect())
        .collect();
    // What the file reaches: the words of its code, then of each body it
    // reaches, until no new body is reached.
    let mut reach: HashSet<&str> = own.words.iter().map(String::as_str).collect();
    let mut reached: Vec<Vec<bool>> = keep.iter().map(|k| vec![false; k.len()]).collect();
    loop {
        let mut more = false;
        for (i, (_, o)) in others.iter().enumerate() {
            for (j, b) in o.bodies.iter().enumerate() {
                let Some(name) = &b.name else {
                    continue;
                };
                if !reached[i][j] && reach.contains(name.as_str()) {
                    reached[i][j] = true;
                    keep[i][j] = true;
                    reach.extend(b.words.iter().map(String::as_str));
                    more = true;
                }
            }
        }
        if !more {
            break;
        }
    }
    others
        .iter()
        .zip(&keep)
        .map(|((text, o), keep)| blank_bodies(text, o, keep))
        .collect()
}

/// `text` with each body not kept left out: only its line breaks stay,
/// before its closing `}`.
fn blank_bodies(text: &str, o: &Outline, keep: &[bool]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for (b, keep) in o.bodies.iter().zip(keep) {
        let code = &text[b.from..b.to];
        let lines = code.matches('\n').count();
        let value = if b.returns { "panic(\"\")" } else { "" };
        if *keep || b.from < at {
            continue;
        }
        out.push_str(&text[at..b.from]);
        out.push('{');
        out.push_str(value);
        out.extend(std::iter::repeat_n('\n', lines));
        out.push('}');
        at = b.to;
    }
    out.push_str(&text[at..]);
    out
}

/// The declarations of a project's open files when it was last checked
/// whole: while each is still the same, a change is checked quickly.
pub(crate) struct Whole {
    /// The project's `src/main.kav`, made absolute.
    main: PathBuf,
    skeletons: HashMap<PathBuf, String>,
}

impl Whole {
    /// The open files of the project of the file at `path`, as `server`
    /// holds them now: just checked whole.
    pub(crate) fn of(server: &crate::Server, path: &str) -> Option<Whole> {
        let main = project_main(Path::new(path))?;
        let main = std::fs::canonicalize(main).ok()?;
        let src = main.parent()?;
        let mut outlines = server.outlines.borrow_mut();
        let skeletons = (server.docs.values())
            .filter_map(|d| {
                let full = d.canonical.as_ref().filter(|c| c.starts_with(src))?;
                let skeleton = outlines.of(d.path.as_ref()?, &d.text).skeleton.clone();
                Some((full.clone(), skeleton))
            })
            .collect();
        Some(Whole { main, skeletons })
    }
}

impl crate::Server {
    /// Whether the document `uri` may be checked quickly: its project was
    /// checked whole, and no open file of it has changed a declaration
    /// since. Other files' messages then stay as they are.
    pub(crate) fn quick(&self, uri: &str) -> bool {
        let (Some(doc), Some(whole)) = (self.docs.get(uri), &self.whole) else {
            return false;
        };
        let main = (doc.path.as_deref())
            .and_then(|p| project_main(Path::new(p)))
            .and_then(|m| std::fs::canonicalize(m).ok());
        let Some(src) = main
            .as_ref()
            .filter(|m| **m == whole.main)
            .and_then(|m| m.parent())
        else {
            return false;
        };
        let mut outlines = self.outlines.borrow_mut();
        self.docs.values().all(|d| {
            let Some(full) = d.canonical.as_ref().filter(|c| c.starts_with(src)) else {
                return true; // another project's, or a file alone
            };
            let now = d.path.as_ref().map(|p| &outlines.of(p, &d.text).skeleton);
            now.is_some() && now == whole.skeletons.get(full)
        })
    }
}

#[cfg(test)]
mod tests;
