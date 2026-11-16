//! `kavach run --hot` (spec 10.5): functions swapped while the program
//! runs, keeping its values. Each change checked is a new *generation* of
//! the program. The interpreter knows things by where they are written,
//! and a change moves them, so each function body and each lambda runs
//! with the generation it was written in: a call switches to its
//! function's, and back when it returns. Calls find the newest version of
//! a function by its definition, which stays the same while only bodies
//! change; a change of anything else restarts the program.

use crate::interp::Interp;
use crate::value::{Eval, Flow, Value};
use kavach_ast::{FnDecl, ItemKind, Span, TypeBody};
use kavach_driver::Compiled;
use kavach_resolver::DefId;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// What `kavach run --hot` gives the interpreter: a flag set when a file
/// changed, how to check the program again, with the files that changed
/// (`None` when it has errors, which the loader shows), and a flag the
/// interpreter sets when the change needs a restart.
pub struct Reloader {
    pub changed: Arc<AtomicBool>,
    pub load: Box<dyn FnMut() -> Option<(&'static Compiled, String)> + Send>,
    pub restart: Arc<AtomicBool>,
}

/// The generations of a program running hot.
pub(crate) struct Hot<'a> {
    /// Each generation, with its map from where a name is defined to its
    /// definition; the running one's map is the interpreter's own.
    gens: Vec<(&'a Compiled, Option<HashMap<u32, DefId>>)>,
    /// Each function's generation, by its declaration's address.
    owners: HashMap<*const FnDecl, usize>,
    pub(crate) running: usize,
    reloader: Reloader,
}

impl<'a> Hot<'a> {
    pub(crate) fn new(first: &'a Compiled, reloader: Reloader) -> Self {
        let mut hot = Hot {
            gens: vec![(first, None)],
            owners: HashMap::new(),
            running: 0,
            reloader,
        };
        hot.own(first, 0);
        hot
    }

    /// Notes every function of `c`, methods too, as generation `g`'s.
    fn own(&mut self, c: &'a Compiled, g: usize) {
        for item in &c.file.items {
            let methods = match &item.kind {
                ItemKind::Fn(f) => {
                    self.owners.insert(f as *const FnDecl, g);
                    continue;
                }
                ItemKind::Type(t) => match &t.body {
                    TypeBody::Record(r) => &r.methods,
                    _ => continue,
                },
                ItemKind::Extend(e) => &e.methods,
                _ => continue,
            };
            for m in methods {
                if let ItemKind::Fn(f) = &m.kind {
                    self.owners.insert(f as *const FnDecl, g);
                }
            }
        }
    }
}

/// `c`'s source with every function body emptied: two generations whose
/// shapes are equal differ only in their bodies, so their definitions
/// and values match.
fn shape(c: &Compiled) -> String {
    let mut bodies: Vec<Span> = Vec::new();
    let mut add = |f: &FnDecl| {
        if let Some(b) = &f.body {
            bodies.push(b.span);
        }
    };
    for item in &c.file.items {
        match &item.kind {
            ItemKind::Fn(f) => add(f),
            ItemKind::Type(t) => {
                if let TypeBody::Record(r) = &t.body {
                    r.methods.iter().for_each(|m| {
                        if let ItemKind::Fn(f) = &m.kind {
                            add(f);
                        }
                    });
                }
            }
            ItemKind::Extend(e) => e.methods.iter().for_each(|m| {
                if let ItemKind::Fn(f) = &m.kind {
                    add(f);
                }
            }),
            _ => {}
        }
    }
    bodies.sort_by_key(|s| s.start);
    let (mut out, mut at) = (String::new(), 0usize);
    for b in bodies {
        let (start, end) = (b.start as usize, b.end as usize);
        out.push_str(c.src.get(at..start).unwrap_or(""));
        out.push_str("{}");
        at = end;
    }
    out.push_str(c.src.get(at..).unwrap_or(""));
    out
}

impl<'a> Interp<'a> {
    /// Makes generation `g` the running one, giving the one before.
    fn switch(&mut self, g: usize) -> usize {
        let Some(hot) = self.hot.as_mut() else {
            return 0;
        };
        let before = hot.running;
        if g != before {
            let running = std::mem::take(&mut self.decl);
            hot.gens[before].1 = Some(running);
            self.decl = hot.gens[g].1.take().unwrap_or_default();
            self.c = hot.gens[g].0;
            hot.running = g;
        }
        before
    }

    /// When a file changed: checks the program again, and with only its
    /// bodies changed, makes the new functions the ones calls find; with
    /// more changed, asks for a restart and ends the program.
    fn reload(&mut self) -> Result<(), Flow<'a>> {
        let Some(hot) = self.hot.as_mut() else {
            return Ok(());
        };
        if !hot.reloader.changed.swap(false, Ordering::AcqRel) {
            return Ok(());
        }
        let Some((new, files)) = (hot.reloader.load)() else {
            return Ok(());
        };
        let newest = hot.gens.last().map_or(self.c, |g| g.0);
        if shape(newest) != shape(new) {
            hot.reloader.restart.store(true, Ordering::Release);
            return Err(Flow::Exit(0));
        }
        let g = hot.gens.len();
        let decl = (new.resolution.defs.iter().enumerate())
            .filter_map(|(id, d)| d.span.map(|s| (s.start, id)))
            .collect();
        hot.gens.push((new, Some(decl)));
        hot.own(new, g);
        // The new generation's functions, types and methods, by the
        // definitions they share with the old.
        let before = self.switch(g);
        self.collect();
        self.collect_std_modules();
        self.switch(before);
        eprintln!("-- changed: {files}, functions swapped");
        Ok(())
    }

    /// Calls `f` as `call_fn_plain` does; running hot, after any change
    /// waiting, the newest version of `f`, in its own generation.
    pub(crate) fn call_fn(
        &mut self,
        f: &'a FnDecl,
        self_value: Option<Value<'a>>,
        args: Vec<(Option<&str>, Value<'a>)>,
        span: Span,
    ) -> Eval<'a> {
        if self.hot.is_none() {
            return self.call_fn_plain(f, self_value, args, span);
        }
        self.reload()?;
        let f = self.newest(f);
        let g = (self.hot.as_ref()).and_then(|h| h.owners.get(&(f as *const FnDecl)).copied());
        let before = self.switch(g.unwrap_or(0));
        let result = self.call_fn_plain(f, self_value, args, span);
        self.switch(before);
        result
    }

    /// The newest version of `f`: the function now defined where `f` was.
    fn newest(&self, f: &'a FnDecl) -> &'a FnDecl {
        let Some(hot) = self.hot.as_ref() else {
            return f;
        };
        let Some(&g) = hot.owners.get(&(f as *const FnDecl)) else {
            return f;
        };
        let decl = if g == hot.running {
            Some(&self.decl)
        } else {
            hot.gens[g].1.as_ref()
        };
        let Some(&def) = decl.and_then(|d| d.get(&f.name.span.start)) else {
            return f;
        };
        self.fns.get(&def).copied().unwrap_or(f)
    }

    /// Runs `body` in generation `g`, the one a lambda was made in.
    pub(crate) fn in_generation<T>(&mut self, g: usize, body: impl FnOnce(&mut Self) -> T) -> T {
        if self.hot.is_none() {
            return body(self);
        }
        let before = self.switch(g);
        let result = body(self);
        self.switch(before);
        result
    }

    /// The running generation, for a lambda made now.
    pub(crate) fn generation(&self) -> usize {
        self.hot.as_ref().map_or(0, |h| h.running)
    }
}
