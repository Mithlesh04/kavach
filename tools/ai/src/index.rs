//! The index (spec 13.9): every declaration and reference of a checked
//! program, which `query` and `context` answer from. A project keeps it
//! in `.kavach/index` (`store.rs`), so later calls skip checking.

use crate::outline::outline_item;
use kavach_ast::{AgentMember, FnDecl, Item, ItemKind, Span, TypeBody};
use kavach_driver::Compiled;
use kavach_resolver::DefKind;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Index {
    /// The entry file, as it was named: paths in answers are relative to it.
    pub entry: String,
    /// The compiler version, then each file with a hash of its text.
    pub stamp: Vec<(String, String)>,
    /// In the order they appear in the program.
    pub decls: Vec<Decl>,
    pub refs: Vec<Ref>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Decl {
    /// `fn`, `method`, `type`, `variant`, `interface`, `capability`,
    /// `agent` or `field`.
    pub kind: String,
    pub name: String,
    /// The type, capability or agent a method or field belongs to.
    pub owner: Option<String>,
    pub module: String,
    pub file: String,
    pub line: usize,
    /// A function's signature on one line; empty for other declarations.
    pub signature: String,
    /// The first name of each capability it uses: in its `uses` clause,
    /// or inferred from its body.
    pub uses: Vec<String>,
    /// The error type after `!`.
    pub fails: Option<String>,
    /// The outline (spec 13.3) of a top-level declaration.
    pub outline: String,
    /// The full code of what `context` can show: top-level declarations,
    /// methods and agent handlers.
    pub code: String,
    /// The top-level declarations its code uses, in order of first use.
    pub refers: Vec<usize>,
}

/// One use of a declared name in the program's own code.
#[derive(Debug, Serialize, Deserialize)]
pub struct Ref {
    /// The name, and the module that declares it.
    pub name: String,
    pub module: String,
    pub file: String,
    pub line: usize,
    /// The function the use is in, or `top level`.
    pub within: String,
    /// The text of its line, trimmed.
    pub text: String,
}

impl Decl {
    /// `Order.total` for a method, `refund` for the rest.
    pub fn short_name(&self) -> String {
        match &self.owner {
            Some(owner) => format!("{owner}.{}", self.name),
            None => self.name.clone(),
        }
    }

    /// Whether `name` names it, with or without its module (spec 13.9):
    /// `refund`, `orders.refund`, `Order.total`, `orders.Order.total`.
    pub fn is_named(&self, name: &str) -> bool {
        let short = self.short_name();
        name == short
            || name
                .strip_prefix(self.module.as_str())
                .and_then(|rest| rest.strip_prefix('.'))
                == Some(short.as_str())
    }
}

impl Ref {
    pub fn is_named(&self, name: &str) -> bool {
        self.name == name || format!("{}.{}", self.module, self.name) == name
    }
}

/// Finds file, line and line text of offsets in the program's source.
struct Lines<'a> {
    c: &'a Compiled,
    starts: Vec<u32>,
}

impl<'a> Lines<'a> {
    fn new(c: &'a Compiled) -> Self {
        let mut starts = vec![0];
        starts.extend(c.src.match_indices('\n').map(|(i, _)| i as u32 + 1));
        Lines { c, starts }
    }

    /// The 0-based line of the whole source an offset is on.
    fn global(&self, offset: u32) -> usize {
        self.starts.partition_point(|s| *s <= offset) - 1
    }

    fn at(&self, offset: u32) -> (String, usize) {
        let files = &self.c.files;
        let f = &files[files
            .partition_point(|f| f.start <= offset)
            .saturating_sub(1)];
        (f.path.clone(), self.global(offset) + 2 - f.first_line)
    }

    fn text(&self, offset: u32) -> String {
        let g = self.global(offset);
        let end = self
            .starts
            .get(g + 1)
            .map_or(self.c.src.len(), |e| *e as usize);
        self.c.src[self.starts[g] as usize..end].trim().to_string()
    }
}

impl Index {
    /// The index of a program checked without errors.
    pub fn build(c: &Compiled, entry: &str) -> Index {
        let lines = Lines::new(c);
        let res = &c.resolution;
        let module_of = |at: u32| {
            res.modules
                .get(res.module_at(at))
                .map_or(String::new(), |m| m.name.clone())
        };
        let new_decl = |kind: &str, name: &str, owner: Option<String>, at: u32| {
            let (file, line) = lines.at(at);
            Decl {
                kind: kind.to_string(),
                name: name.to_string(),
                owner,
                module: module_of(at),
                file,
                line,
                ..Decl::default()
            }
        };
        // Each declaration by the offset of its name, with the spans of
        // its code and, for top-level ones, of the whole item.
        let mut found: Vec<(u32, Decl, Option<Span>, Option<Span>)> = Vec::new();
        for d in &res.defs {
            let kind = match d.kind {
                DefKind::Fn => "fn",
                DefKind::Type => "type",
                DefKind::Variant { .. } => "variant",
                DefKind::Interface => "interface",
                DefKind::Capability => "capability",
                DefKind::Agent => "agent",
                _ => continue,
            };
            if let Some(span) = d.span.filter(|s| c.is_own(s.start)) {
                found.push((
                    span.start,
                    new_decl(kind, &d.name, None, span.start),
                    None,
                    None,
                ));
            }
        }
        let functions = functions(c);
        for (f, item, owner) in &functions {
            let at = f.name.span.start;
            let i = match found.iter().position(|x| x.0 == at) {
                Some(i) => i,
                None => {
                    found.push((at, new_decl("method", &f.name.name, None, at), None, None));
                    found.len() - 1
                }
            };
            let d = &mut found[i].1;
            d.owner.clone_from(owner);
            d.signature = signature(c, item);
            // Written in `uses`, or inferred from the body (spec 8.1): `main`
            // printing uses `Console` without saying so.
            d.uses = f.uses.iter().map(|p| p.segments[0].name.clone()).collect();
            let def = c
                .resolution
                .defs
                .iter()
                .position(|x| x.span.is_some_and(|s| s.start == f.name.span.start));
            for cap in def.and_then(|x| c.uses.get(&x)).into_iter().flatten() {
                if !d.uses.contains(cap) {
                    d.uses.push(cap.clone());
                }
            }
            d.fails = f.errors.as_ref().map(|e| e.span.text(&c.src).to_string());
        }
        for item in c.own_items() {
            if let ItemKind::Type(t) = &item.kind
                && let TypeBody::Record(r) = &t.body
            {
                for field in &r.fields {
                    let (name, at) = (&field.name.name, field.name.span.start);
                    let owner = Some(t.name.name.clone());
                    found.push((at, new_decl("field", name, owner, at), None, None));
                }
            }
        }
        for (at, code, whole) in code_spans(c) {
            if let Some(x) = found.iter_mut().find(|x| x.0 == at) {
                x.1.code = c.src[code.start as usize..code.end as usize]
                    .trim_end()
                    .to_string();
                x.2 = Some(code);
                if let Some(item) = whole {
                    x.1.outline = outline_item(c, item);
                    x.3 = Some(item.span);
                }
            }
        }
        found.sort_by_key(|x| x.0);

        let mut uses: Vec<(u32, usize)> = res.uses.iter().map(|(a, d)| (*a, *d)).collect();
        uses.sort_unstable();
        let top_level: HashMap<u32, usize> = found
            .iter()
            .enumerate()
            .filter(|(_, x)| x.3.is_some())
            .map(|(i, x)| (x.0, i))
            .collect();
        let refers: Vec<Vec<usize>> = found
            .iter()
            .map(|x| {
                x.2.map_or(Vec::new(), |code| {
                    referenced(c, &uses, code, &top_level, &found)
                })
            })
            .collect();
        let decls = found
            .into_iter()
            .zip(refers)
            .map(|((_, mut d, _, _), r)| {
                d.refers = r;
                d
            })
            .collect();

        let fn_spans: Vec<(Span, &str)> = functions
            .iter()
            .map(|(f, item, _)| (item.span, f.name.name.as_str()))
            .collect();
        let mut refs = Vec::new();
        for (at, def) in uses.iter().filter(|(at, _)| c.is_own(*at)) {
            let d = &res.defs[*def];
            let Some(span) = d.span else { continue };
            if matches!(d.kind, DefKind::Local { .. } | DefKind::Param) {
                continue;
            }
            let within = fn_spans
                .iter()
                .filter(|(s, _)| s.start <= *at && *at < s.end)
                .map(|(_, name)| *name)
                .next_back()
                .unwrap_or("top level");
            let (file, line) = lines.at(*at);
            refs.push(Ref {
                name: d.name.clone(),
                module: module_of(span.start),
                file,
                line,
                within: within.to_string(),
                text: lines.text(*at),
            });
        }
        Index {
            entry: entry.to_string(),
            stamp: Vec::new(),
            decls,
            refs,
        }
    }
}

/// A function's signature on one line, without doc comments or body.
fn signature(c: &Compiled, item: &Item) -> String {
    let text = outline_item(c, item);
    let code: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with("///"))
        .collect();
    code.join(" ").trim_end_matches("{ … }").trim().to_string()
}

/// Every function, method and agent handler, with the item that shows
/// it and the type, capability or agent it belongs to.
fn functions(c: &Compiled) -> Vec<(&FnDecl, Item, Option<String>)> {
    let mut out = Vec::new();
    for item in c.own_items() {
        let (owner, methods): (String, &[Item]) = match &item.kind {
            ItemKind::Fn(f) => {
                out.push((f, item.clone(), None));
                continue;
            }
            ItemKind::Type(t) => match &t.body {
                TypeBody::Record(r) => (t.name.name.clone(), &r.methods),
                TypeBody::Union(_) | TypeBody::Alias(_) => continue,
            },
            ItemKind::Extend(e) => (e.target.span.text(&c.src).to_string(), &e.methods),
            ItemKind::Capability(cap) => (cap.name.name.clone(), &cap.methods),
            ItemKind::Agent(a) => {
                for m in &a.members {
                    if let AgentMember::On(f) = m {
                        let item = Item {
                            kind: ItemKind::Fn(f.clone()),
                            docs: Vec::new(),
                            is_pub: false,
                            span: handler_span(c, f),
                        };
                        out.push((f, item, Some(a.name.name.clone())));
                    }
                }
                continue;
            }
            _ => continue,
        };
        for m in methods {
            if let ItemKind::Fn(f) = &m.kind {
                out.push((f, m.clone(), Some(owner.clone())));
            }
        }
    }
    out
}

/// An agent handler, from its `on` keyword to the end of its body.
fn handler_span(c: &Compiled, f: &FnDecl) -> Span {
    let start = c.src[..f.name.span.start as usize].rfind("on").unwrap_or(0) as u32;
    Span {
        start,
        end: f.body.as_ref().map_or(f.name.span.end, |b| b.span.end),
    }
}

/// What `context` can show (spec 13.4): the offset of each name, the code,
/// and the item for top-level declarations.
fn code_spans(c: &Compiled) -> Vec<(u32, Span, Option<&Item>)> {
    let mut out = Vec::new();
    for item in c.own_items() {
        let name = match &item.kind {
            ItemKind::Fn(f) => &f.name,
            ItemKind::Type(t) => &t.name,
            ItemKind::Interface(i) => &i.name,
            ItemKind::Capability(cap) => &cap.name,
            ItemKind::Agent(a) => &a.name,
            _ => continue,
        };
        out.push((name.span.start, item.span, Some(item)));
        let methods: &[Item] = match &item.kind {
            ItemKind::Type(t) => match &t.body {
                TypeBody::Record(r) => &r.methods,
                TypeBody::Union(_) | TypeBody::Alias(_) => &[],
            },
            ItemKind::Agent(a) => {
                for m in &a.members {
                    if let AgentMember::On(f) = m {
                        out.push((f.name.span.start, handler_span(c, f), None));
                    }
                }
                &[]
            }
            _ => &[],
        };
        out.extend(methods.iter().filter_map(|m| match &m.kind {
            ItemKind::Fn(f) => Some((f.name.span.start, m.span, None)),
            _ => None,
        }));
    }
    for item in c.own_items() {
        if let ItemKind::Extend(e) = &item.kind {
            out.extend(e.methods.iter().filter_map(|m| match &m.kind {
                ItemKind::Fn(f) => Some((f.name.span.start, m.span, None)),
                _ => None,
            }));
        }
    }
    out
}

/// The top-level declarations used inside `code`, in order of first use,
/// without the one that contains it.
fn referenced(
    c: &Compiled,
    uses: &[(u32, usize)],
    code: Span,
    top_level: &HashMap<u32, usize>,
    found: &[(u32, Decl, Option<Span>, Option<Span>)],
) -> Vec<usize> {
    let res = &c.resolution;
    let from = uses.partition_point(|(at, _)| *at < code.start);
    let mut out = Vec::new();
    for (_, def) in uses[from..].iter().take_while(|(at, _)| *at < code.end) {
        let def = match res.defs[*def].kind {
            DefKind::Variant { of } => of,
            DefKind::Fn
            | DefKind::Type
            | DefKind::Interface
            | DefKind::Capability
            | DefKind::Agent => *def,
            _ => continue,
        };
        let Some(i) = res.defs[def].span.and_then(|s| top_level.get(&s.start)) else {
            continue;
        };
        let item = found[*i].3.unwrap_or(code);
        let contains_code = item.start <= code.start && code.end <= item.end;
        if !contains_code && !out.contains(i) {
            out.push(*i);
        }
    }
    out
}
