//! Projects with several files (spec 10.1–10.3). All files are parsed as
//! one source, so every later stage keeps working on one tree; the file
//! table maps positions back to files, and `orders.total` is rewritten
//! to the name `total` once the resolver has linked it.

use kavach_ast::visit::{VisitMut, walk_expr, walk_type};
use kavach_ast::{Expr, ExprKind, File, TypeExpr, TypeKind};
use kavach_resolver::{DefKind, Resolution};
use std::path::{Path, PathBuf};

/// One file of a program: its path, its module name and its text, and
/// the package it belongs to: empty for the program's own files, else a
/// dependency's name (spec 10.12).
#[derive(Clone, Debug, Default)]
pub struct Source {
    pub path: String,
    pub module: String,
    pub text: String,
    pub package: String,
}

/// Where a file starts in the combined source.
#[derive(Clone, Debug)]
pub struct SourceFile {
    pub path: String,
    pub start: u32,
    /// The combined source's line where this file's first line is.
    pub first_line: usize,
}

pub(crate) fn kav_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            kav_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "kav") {
            out.push(p);
        }
    }
}

/// The files of the project `entry` belongs to, entry first: every `.kav`
/// file under the project's `src/`. `None` if `entry` is not in a
/// project's `src/` folder.
pub fn project_sources(entry: &Path) -> Option<Vec<Source>> {
    let full = std::fs::canonicalize(entry).ok()?;
    let root = full.ancestors().find(|d| d.join("kavach.toml").is_file())?;
    let src = root.join("src");
    let rel = full.strip_prefix(&src).ok()?;
    // `orders.test.kav` is the tests of `orders` (spec 11.8): the same module.
    let module_of = |rel: &Path| {
        let name = rel
            .with_extension("")
            .to_string_lossy()
            .replace(['/', '\\'], ".");
        let name = name.strip_suffix(".test").unwrap_or(&name);
        name.trim_start_matches('.').to_string()
    };
    let mut files = Vec::new();
    kav_files(&src, &mut files);
    let mut sources = vec![Source {
        path: entry.to_string_lossy().to_string(),
        module: module_of(rel),
        text: std::fs::read_to_string(&full).ok()?,
        package: String::new(),
    }];
    for f in files.into_iter().filter(|f| *f != full) {
        let r = f.strip_prefix(&src).ok()?.to_path_buf();
        sources.push(Source {
            path: r.to_string_lossy().replace('\\', "/").to_string(),
            module: module_of(&r),
            text: std::fs::read_to_string(&f).ok()?,
            package: String::new(),
        });
    }
    // Each test file right after its module's file, so the two make one
    // range of the source (`compile_sources`).
    let is_test = |s: &Source| s.path.ends_with(".test.kav");
    let (tests, mut ordered): (Vec<Source>, Vec<Source>) = sources.into_iter().partition(is_test);
    for t in tests {
        match ordered.iter().position(|s| s.module == t.module) {
            Some(i) => ordered.insert(i + 1, t),
            None => ordered.push(t),
        }
    }
    let mut sources = ordered;
    let local = sources.len();
    sources.extend(crate::deps::sources(root));
    // Paths in messages are relative to where the entry was named from.
    let shown = entry.parent().map(Path::to_path_buf).unwrap_or_default();
    let base = std::fs::canonicalize(&shown).unwrap_or(shown.clone());
    for s in sources.iter_mut().take(local).skip(1) {
        let full = src.join(&s.path);
        s.path = match full.strip_prefix(&base) {
            Ok(r) => shown.join(r).to_string_lossy().replace('\\', "/"),
            Err(_) => full.to_string_lossy().to_string(),
        };
    }
    Some(sources)
}

/// Rewrites `orders.total` and `orders.Order` (in values and in types) to
/// the plain names, which the resolver has already linked.
struct Qualified<'r> {
    res: &'r Resolution,
}

impl Qualified<'_> {
    fn is_module(&self, ident: &kavach_ast::Ident) -> bool {
        self.res
            .uses
            .get(&ident.span.start)
            .is_some_and(|d| self.res.defs[*d].kind == DefKind::Module)
    }

    /// `math.gcd` where `use std.math` names a standard library module
    /// written in Kavach that has `gcd` (the resolver linked it); not
    /// `math.sqrt`, which the runtime provides.
    fn is_std_member(&self, module: &kavach_ast::Ident, name: &kavach_ast::Ident) -> bool {
        let std = self.res.uses.get(&module.span.start);
        std.is_some_and(|d| self.res.module_defs.contains_key(d))
            && self.res.uses.contains_key(&name.span.start)
    }
}

impl VisitMut for Qualified<'_> {
    fn expr(&mut self, e: &mut Expr) {
        if let ExprKind::Field { base, name, .. } = &e.kind
            && let ExprKind::Name(id) = &base.kind
            && (self.is_module(id) || self.is_std_member(id, name))
        {
            e.kind = ExprKind::Name(name.clone());
            return;
        }
        walk_expr(self, e);
    }

    fn type_expr(&mut self, t: &mut TypeExpr) {
        if let TypeKind::Named { path, .. } = &mut t.kind
            && path.segments.len() > 1
            && self.is_module(&path.segments[0])
        {
            let last = path.segments.pop().expect("a name");
            path.segments = vec![last];
        }
        walk_type(self, t);
    }
}

pub(crate) fn unqualify(file: &mut File, res: &Resolution) {
    kavach_ast::visit::walk_file(&mut Qualified { res }, file);
}
