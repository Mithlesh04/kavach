//! Generated tests (spec 11.2–11.4, 11.10): every function is called with
//! generated arguments that meet its `require`; a panic, broken `ensure`
//! or broken invariant is a failure, shrunk to the smallest input, saved
//! as a found case and shown with fix suggestions.

mod fixes;
mod found;
mod keep;

pub use keep::Kept;
mod shrink;
mod targets;
pub(crate) mod values;

use crate::interp::Interp;
use crate::value::{Flow, Value};
use found::{Reader, can_save, write_case};
use kavach_ast::{FnDecl, ItemKind, Span, TypeBody};
use std::path::PathBuf;
use targets::Target;
use values::{Gen, Pick, Rng};

/// How `kavach test` generates cases (spec 11.9).
#[derive(Clone, Debug)]
pub struct TestOptions {
    pub seed: u64,
    /// Generated cases per function.
    pub cases: usize,
    /// The time budget of one call, counted in loop rounds and calls, so
    /// a run repeats exactly.
    pub step_budget: u64,
    /// Where the entry's found cases are read from and saved; another
    /// module's are beside it, named after the module (`orders.kavf`).
    /// `None` keeps them only in the report.
    pub found_file: Option<PathBuf>,
}

impl Default for TestOptions {
    fn default() -> Self {
        TestOptions {
            seed: 1,
            cases: 200,
            step_budget: 1_000_000,
            found_file: None,
        }
    }
}

enum Outcome {
    Pass,
    /// The input broke the function's own `require`: not a valid case.
    Invalid,
    Fail(String, Span),
    /// The call reached a C function, which runs only in native code.
    Native,
}

impl<'a> Interp<'a> {
    /// Calls `t` with `args`, its output hidden and its time limited.
    fn run_case(&mut self, t: &Target<'a>, args: &[Value<'a>], budget: u64) -> Outcome {
        self.reached_c = false;
        let called = self.call_case(t, args, budget);
        if std::mem::take(&mut self.reached_c) {
            return Outcome::Native;
        }
        match called {
            Err(Flow::Panic(_, span)) if t.f.requires.iter().any(|r| r.span == span) => {
                Outcome::Invalid
            }
            Err(Flow::Panic(message, span)) => Outcome::Fail(message, span),
            _ => Outcome::Pass,
        }
    }

    /// Calls `t` with `args`, its output hidden and its time limited, and
    /// gives what the call gave.
    fn call_case(
        &mut self,
        t: &Target<'a>,
        args: &[Value<'a>],
        budget: u64,
    ) -> Result<Value<'a>, Flow<'a>> {
        let (out_len, frames) = (self.out.len(), self.frames.len());
        crate::run::running(Some(format!("the generated test of `{}`", t.name)));
        self.testing = true;
        self.steps = 0;
        self.step_limit = Some(budget);
        let (self_value, rest) = match t.has_self {
            true => (args.first().cloned(), &args[1..]),
            false => (None, args),
        };
        let call_args = rest.iter().map(|v| (None, v.clone())).collect();
        // Fake answers come from the arguments: the same case gets the
        // same answers again (when shrinking, or replaying found cases).
        self.dry_calls = args.iter().fold(0x5EED, |h: u64, v| {
            v.display_code()
                .bytes()
                .fold(h, |h, b| h.wrapping_mul(31).wrapping_add(u64::from(b)))
        });
        let world = crate::fakes::World::new(self.dry_calls);
        self.random = Some(Rng::new(self.dry_calls));
        self.fake = Some(std::rc::Rc::new(std::cell::RefCell::new(world)));
        let result = self.call_fn(t.f, self_value, call_args, t.f.name.span);
        self.fake = None;
        self.random = None;
        self.step_limit = None;
        self.testing = false;
        self.out.truncate(out_len);
        self.frames.truncate(frames);
        crate::run::running(None);
        result
    }

    /// The smallest arguments that still fail at the same place.
    fn shrink(
        &mut self,
        t: &Target<'a>,
        mut args: Vec<Value<'a>>,
        at: Span,
        budget: u64,
    ) -> Vec<Value<'a>> {
        let mut calls = 0;
        'outer: while calls < 2_000 {
            for p in 0..args.len() {
                let mut candidates = self.smaller(&args[p]);
                if matches!(t.params[p], Gen::Optional(_)) && args[p] != Value::None {
                    candidates.insert(0, Value::None);
                }
                for c in candidates {
                    calls += 1;
                    let mut tried = args.clone();
                    tried[p] = c;
                    if let Outcome::Fail(_, span) = self.run_case(t, &tried, budget)
                        && span == at
                    {
                        args = tried;
                        continue 'outer;
                    }
                }
            }
            break;
        }
        args
    }

    /// Prints a failure: what broke, where, the smallest input and fixes.
    fn report_failure(&mut self, t: &Target<'a>, message: &str, at: Span, args: &[Value<'a>]) {
        let (path, line) = self.c.shown_location(at.start);
        let code = self.text(at).lines().next().unwrap_or("").trim();
        let mut text = format!("✗ {}: panicked ({message})\n", t.name);
        text.push_str(&format!("    {path}:{line}   {code}\n"));
        let names: Vec<&str> = t.f.params.iter().map(|p| p.name.name.as_str()).collect();
        let shown: Vec<String> = names
            .iter()
            .zip(args)
            .map(|(n, v)| format!("{n} = {}", self.code_of(v)))
            .collect();
        text.push_str(&format!("  smallest failing input: {}\n", shown.join(", ")));
        let ret = t.f.ret.as_ref().map_or("()", |r| self.text(r.span));
        text.push_str("  How to fix (pick one):\n");
        for (n, fix) in fixes::suggest(message, code, ret).iter().enumerate() {
            text.push_str(&format!("    {}. {fix}\n", n + 1));
        }
        self.out.push_str(&text);
    }

    /// Replays found cases, then generates cases for every function.
    /// Returns the number of functions that failed.
    pub(crate) fn generated_tests(&mut self, opts: &TestOptions) -> usize {
        let (targets, mut skipped) = self.targets();
        let mut native = 0;
        self.out.push_str(&format!("seed {}\n", opts.seed));
        // Each module's found cases, read once; the new ones, by file.
        let mut saved: std::collections::HashMap<PathBuf, Vec<String>> = Default::default();
        let mut new_found: Vec<(PathBuf, String)> = Vec::new();
        let (mut failed, mut weak) = (0, Vec::new());
        let mut rng = Rng::new(opts.seed);
        for t in &targets {
            // Found cases first, so a fixed bug stays fixed (11.4).
            let mut failure = None;
            let file = self.found_file(opts, t);
            let saved = match &file {
                Some(f) => &*saved
                    .entry(f.clone())
                    .or_insert_with(|| Self::saved_cases(f)),
                None => &[][..],
            };
            for line in saved
                .iter()
                .filter(|l| l.starts_with(&format!("{}(", t.name)))
            {
                if let Some(args) = self.read_case(t, line)
                    && let Outcome::Fail(m, at) = self.run_case(t, &args, opts.step_budget)
                {
                    failure = Some((m, at, args));
                    break;
                }
            }
            let mut valid = 0;
            let mut k = 0;
            let mut reaches_c = false;
            let cases = if t.params.is_empty() { 1 } else { opts.cases };
            while failure.is_none() && !reaches_c && valid < cases && k < cases * 5 {
                let pick = if k < 40 { Pick::Edge(k) } else { Pick::Random };
                k += 1;
                let args: Option<Vec<Value<'a>>> = t
                    .params
                    .iter()
                    .enumerate()
                    .map(|(n, g)| {
                        let p = match pick {
                            Pick::Edge(i) => Pick::Edge(i + n * 3),
                            Pick::Random => Pick::Random,
                        };
                        self.generate(g, p, &t.near, &mut rng, 0)
                    })
                    .collect();
                let Some(args) = args else { continue };
                match self.run_case(t, &args, opts.step_budget) {
                    Outcome::Pass => valid += 1,
                    Outcome::Invalid => {}
                    Outcome::Native => reaches_c = true,
                    Outcome::Fail(m, at) => {
                        let small = self.shrink(t, args, at, opts.step_budget);
                        let line = write_case(self, &t.name, &small);
                        if let (Some(f), true) = (&file, small.iter().all(can_save))
                            && !saved.contains(&line)
                        {
                            new_found.push((f.clone(), line));
                        }
                        failure = Some((m, at, small));
                    }
                }
            }
            if reaches_c {
                let why = "reaches C, which runs only in native code (10.7)";
                skipped.push(format!("{}: {why}", t.name));
                native += 1;
                continue;
            }
            match failure {
                Some((m, at, args)) => {
                    failed += 1;
                    self.report_failure(t, &m, at, &args);
                }
                // Its examples ran with the hand-written checks above.
                None if valid == 0 && !t.f.examples.is_empty() => {
                    let why = match t.f.requires.is_empty() {
                        true => "no input could be generated",
                        false => "no generated input met its `require`",
                    };
                    let name = &t.name;
                    let line = format!("✓ {name}  checked by its examples only: {why}\n");
                    self.out.push_str(&line);
                }
                None if valid == 0 && !t.f.requires.is_empty() => {
                    self.out.push_str(&format!(
                        "! {}: no generated input met its `require`; add an `example` (11.3)\n",
                        t.name
                    ));
                }
                // Never "✓ … 0 cases": nothing was checked.
                None if valid == 0 => {
                    self.out.push_str(&format!(
                        "! {}: no input could be generated; add an `example`, or a `Sample` for its types (11.3)\n",
                        t.name
                    ));
                }
                None => {
                    self.out.push_str(&format!("✓ {}  {valid} cases\n", t.name));
                    if t.f.ensures.is_empty() && t.f.examples.is_empty() {
                        let (path, line) = self.c.shown_location(t.f.name.span.start);
                        weak.push(format!("{path}:{line}   {}", t.name));
                    }
                }
            }
        }
        self.save_found(&new_found);
        let checked = targets.len() - native;
        self.out.push_str(&format!(
            "{checked} functions   {} ✓   {failed} ✗   {} weakly tested\n",
            checked - failed,
            weak.len()
        ));
        if !weak.is_empty() {
            self.out
                .push_str("weakly tested (no contract or example; only checked for panics):\n");
            for w in weak {
                self.out.push_str(&format!("  {w}\n"));
            }
        }
        for s in skipped {
            self.out.push_str(&format!("  not generated: {s}\n"));
        }
        let missed = self.unchecked_modules(&targets);
        if !missed.is_empty() {
            self.out
                .push_str("nothing checked by hand (no example, ensure, invariant or test) in:\n");
            for m in missed {
                self.out.push_str(&format!("  {m}\n"));
            }
        }
        failed
    }

    /// The files of the modules with functions to test, and nothing in
    /// them a person wrote to check them: no `example`, `ensure`,
    /// `invariant` or `test` block, in the module or its `name.test.kav`
    /// (spec 11.9, 11.8).
    fn unchecked_modules(&self, targets: &[Target<'a>]) -> Vec<String> {
        let res = &self.c.resolution;
        let mut tested: Vec<usize> = targets
            .iter()
            .map(|t| res.module_at(t.f.name.span.start))
            .collect();
        tested.sort();
        tested.dedup();
        // As for "weakly tested": a `require` limits the inputs, but
        // checks no result.
        let checks = |f: &FnDecl| !(f.ensures.is_empty() && f.examples.is_empty());
        let mut checked = std::collections::HashSet::new();
        for item in &self.c.file.items {
            let has = match &item.kind {
                ItemKind::Fn(f) => checks(f),
                ItemKind::Test(_) => true,
                ItemKind::Type(t) => match &t.body {
                    TypeBody::Record(r) => {
                        !r.invariants.is_empty()
                            || r.methods
                                .iter()
                                .any(|m| matches!(&m.kind, ItemKind::Fn(f) if checks(f)))
                    }
                    _ => false,
                },
                ItemKind::Extend(e) => e
                    .methods
                    .iter()
                    .any(|m| matches!(&m.kind, ItemKind::Fn(f) if checks(f))),
                _ => false,
            };
            if has {
                checked.insert(res.module_at(item.span.start));
            }
        }
        tested
            .into_iter()
            .filter(|m| !checked.contains(m))
            .filter_map(|m| res.modules.get(m))
            .map(|m| self.c.shown_location(m.start).0)
            .collect()
    }

    fn read_case(&self, t: &Target<'a>, line: &str) -> Option<Vec<Value<'a>>> {
        let mut r = Reader::new(line);
        if r.name() != t.name {
            return None;
        }
        r.expect("(")?;
        let mut args = Vec::new();
        for (n, g) in t.params.iter().enumerate() {
            if n > 0 {
                r.expect(", ")?;
            }
            args.push(r.value(g, self)?);
        }
        r.expect(")")?;
        r.is_done().then_some(args)
    }
}
