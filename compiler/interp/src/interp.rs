//! The interpreter's state, function calls with contracts, and the two
//! ways to run a file: `main` and tests.

use crate::value::{Eval, Field, Flow, Value};
use kavach_ast::*;
use kavach_checker::Prim;
use kavach_driver::Compiled;
use kavach_resolver::{DefId, DefKind};
use std::collections::HashMap;
use std::rc::Rc;

/// One `example` or `test`: true when it passes.
type TestCase<'a> = Box<dyn Fn(&mut Interp<'a>) -> Result<bool, Flow<'a>> + 'a>;

pub(crate) struct Frame<'a> {
    pub(crate) vars: HashMap<DefId, Value<'a>>,
    pub(crate) self_value: Option<Value<'a>>,
    pub(crate) its: Vec<Value<'a>>,
    /// The tasks spawned by this call, waited for when it returns
    /// (spec 9.2).
    pub(crate) spawned: Vec<std::sync::Arc<crate::tasks::TaskHandle<'a>>>,
    /// The type arguments of this call of a generic function
    /// (`generics.rs`).
    pub(crate) generics: HashMap<DefId, kavach_checker::Ty>,
}

pub(crate) struct Interp<'a> {
    pub(crate) c: &'a Compiled,
    pub(crate) decl: HashMap<u32, DefId>,
    pub(crate) fns: HashMap<DefId, &'a FnDecl>,
    pub(crate) records: HashMap<DefId, &'a TypeDecl>,
    pub(crate) unions: HashMap<DefId, &'a TypeDecl>,
    /// Agents and declared capabilities (spec 8.2, 8.3).
    pub(crate) agents: HashMap<DefId, &'a AgentDecl>,
    pub(crate) capabilities: HashMap<DefId, &'a CapabilityDecl>,
    /// The agents whose handlers are running, innermost last: capability
    /// calls go to the last one's implementations.
    pub(crate) running: Vec<Rc<crate::agents::Running<'a>>>,
    /// How deep in capability implementations the running agent is: their
    /// clock reads and waits aren't journaled, the call's outcome is.
    pub(crate) in_call: usize,
    /// While a `limit` rule is checked: the totals `sum` and `count` see.
    pub(crate) totals: Option<crate::guard::Totals<'a>>,
    /// `kavach run --dry-run` (spec 8.5), and how many calls it has
    /// answered (each answer's seed).
    pub(crate) dry_run: bool,
    pub(crate) dry_calls: u64,
    /// `[log]` from `kavach.toml` (spec 14.12), read on the first event.
    pub(crate) log: Option<kavach_runtime_core::log::Settings>,
    /// Methods by type: records and unions by definition, built-in types
    /// extended with `extend` by name.
    pub(crate) methods: HashMap<DefId, HashMap<&'a str, &'a FnDecl>>,
    pub(crate) ext_methods: HashMap<String, HashMap<&'a str, &'a FnDecl>>,
    pub(crate) frames: Vec<Frame<'a>>,
    /// `result` while checking `ensure` (spec 7.2).
    pub(crate) result: Vec<Value<'a>>,
    /// The record whose `invariant` is being checked (spec 7.3).
    pub(crate) invariant_fields: Vec<Vec<Field<'a>>>,
    pub(crate) out: String,
    pub(crate) err: String,
    /// Loop rounds and calls so far, and the most allowed (generated
    /// tests give each call a budget, spec 11.9).
    pub(crate) steps: u64,
    pub(crate) step_limit: Option<u64>,
    /// Names brought in by `use std.<module>`, and the module (spec 14).
    pub(crate) std_modules: HashMap<DefId, String>,
    /// The program's command-line arguments (`cli.args`, spec 14.6).
    pub(crate) args: Vec<String>,
    /// Running a generated test: the terminal gives no input.
    pub(crate) testing: bool,
    /// The tasks of this run, and which one this is (spec 9). `None`
    /// runs a spawned task at once, as before tasks took turns.
    pub(crate) tasks: Option<crate::tasks::Tasks<'a>>,
    pub(crate) task: kavach_runtime_core::tasks::TaskId,
    /// The folders file access is kept inside (`uses Fs("./data")`, spec
    /// 8.1), resolved: one for each such call running.
    pub(crate) fs_scopes: Vec<std::path::PathBuf>,
    /// Where `std.test`'s `Gen` choices come from (`test_random`, spec
    /// 14.7): seeded from the test run for each `sample` call.
    pub(crate) gen_rng: crate::testing::values::Rng,
    /// Where `random_bits` comes from (spec 14.15): `None` for the
    /// system's secure generator; under `kavach test`, the `Random` fake,
    /// seeded for each test and case so it can be repeated (11.5).
    pub(crate) random: Option<crate::testing::values::Rng>,
    /// The `inout` parameters of the call that just returned, for its
    /// caller to write back (spec 3.9): name, position, last value.
    pub(crate) inout_out: Vec<(&'a str, usize, Value<'a>)>,
    /// The type arguments for the generic function about to be called.
    pub(crate) next_generics: Option<HashMap<DefId, kavach_checker::Ty>>,
    /// A generated test's files and clock (spec 11.5), shared by its tasks.
    pub(crate) fake: Option<Rc<std::cell::RefCell<crate::fakes::World>>>,
    /// A test's frozen clock (spec 14.4), shared by its tasks.
    pub(crate) frozen: crate::std_time::Frozen,
    /// Values whose last holder has ended, waiting for their `drop`
    /// (`drops.rs`); tasks share it.
    pub(crate) drops: Rc<std::cell::RefCell<Vec<Value<'a>>>>,
    /// The record types that implement `Drop`.
    pub(crate) drop_types: std::collections::HashSet<DefId>,
    /// The variables each running block made, innermost last.
    pub(crate) block_lets: Vec<Vec<DefId>>,
    /// Inside a `drop`: the queued values before this index wait for it.
    pub(crate) drop_floor: usize,
    /// The journaled events (spec 8.5) this task is in, innermost last:
    /// the calls it makes go to the journal under the last one.
    pub(crate) events: Vec<i64>,
}

impl<'a> Interp<'a> {
    pub(crate) fn new(c: &'a Compiled) -> Self {
        crate::json::set_json_type(c.resolution.builtin_type("std.json", "Json"));
        let decl = c
            .resolution
            .defs
            .iter()
            .enumerate()
            .filter_map(|(id, d)| d.span.map(|s| (s.start, id)))
            .collect();
        let mut i = Interp {
            c,
            decl,
            fns: HashMap::new(),
            records: HashMap::new(),
            unions: HashMap::new(),
            agents: HashMap::new(),
            capabilities: HashMap::new(),
            running: Vec::new(),
            in_call: 0,
            totals: None,
            dry_run: false,
            dry_calls: 0,
            log: None,
            methods: HashMap::new(),
            ext_methods: HashMap::new(),
            frames: vec![Frame {
                vars: HashMap::new(),
                self_value: None,
                its: Vec::new(),
                spawned: Vec::new(),
                generics: HashMap::new(),
            }],
            result: Vec::new(),
            invariant_fields: Vec::new(),
            out: String::new(),
            err: String::new(),
            steps: 0,
            step_limit: None,
            std_modules: HashMap::new(),
            args: Vec::new(),
            testing: false,
            tasks: None,
            task: 0,
            fs_scopes: Vec::new(),
            gen_rng: crate::testing::values::Rng::new(0),
            random: None,
            inout_out: Vec::new(),
            next_generics: None,
            fake: None,
            frozen: None,
            drops: Rc::new(std::cell::RefCell::new(Vec::new())),
            drop_types: std::collections::HashSet::new(),
            block_lets: Vec::new(),
            drop_floor: 0,
            events: Vec::new(),
        };
        i.collect();
        i.collect_std_modules();
        i
    }

    fn collect(&mut self) {
        let fn_items = |methods: &'a [Item]| -> HashMap<&'a str, &'a FnDecl> {
            methods
                .iter()
                .filter_map(|m| match &m.kind {
                    ItemKind::Fn(f) => Some((f.name.name.as_str(), f)),
                    _ => None,
                })
                .collect()
        };
        for item in &self.c.file.items {
            match &item.kind {
                ItemKind::Fn(f) => {
                    if let Some(&def) = self.decl.get(&f.name.span.start) {
                        self.fns.insert(def, f);
                    }
                }
                ItemKind::Type(t) => {
                    let Some(&def) = self.decl.get(&t.name.span.start) else {
                        continue;
                    };
                    match &t.body {
                        TypeBody::Record(r) => {
                            self.records.insert(def, t);
                            let drops = t.implements.iter().any(|i| {
                                matches!(&i.kind, TypeKind::Named { path, .. }
                                    if path.segments.last().is_some_and(|s| s.name == "Drop"))
                            });
                            if drops {
                                self.drop_types.insert(def);
                            }
                            self.methods
                                .entry(def)
                                .or_default()
                                .extend(fn_items(&r.methods));
                        }
                        TypeBody::Alias(_) => {}
                        TypeBody::Union(_) => {
                            self.unions.insert(def, t);
                        }
                    }
                }
                ItemKind::Extend(e) => {
                    let TypeKind::Named { path, .. } = &e.target.kind else {
                        continue;
                    };
                    let first = &path.segments[0];
                    let Some(&def) = self.c.resolution.uses.get(&first.span.start) else {
                        continue;
                    };
                    if self.c.resolution.defs[def].kind == DefKind::Builtin {
                        self.ext_methods
                            .entry(first.name.clone())
                            .or_default()
                            .extend(fn_items(&e.methods));
                    } else {
                        self.methods
                            .entry(def)
                            .or_default()
                            .extend(fn_items(&e.methods));
                    }
                }
                ItemKind::Agent(a) => {
                    if let Some(&def) = self.decl.get(&a.name.span.start) {
                        self.agents.insert(def, a);
                    }
                }
                ItemKind::Capability(c) => {
                    if let Some(&def) = self.decl.get(&c.name.span.start) {
                        self.capabilities.insert(def, c);
                    }
                }
                _ => {}
            }
        }
    }

    /// Counts one loop round or call against the time budget.
    pub(crate) fn step(&mut self, span: Span) -> Result<(), Flow<'a>> {
        self.steps += 1;
        self.stop_point()?;
        match self.step_limit {
            Some(limit) if self.steps > limit => {
                Err(self.panic("did not finish within its time budget (spec 11.9)", span))
            }
            _ => Ok(()),
        }
    }

    pub(crate) fn frame(&mut self) -> &mut Frame<'a> {
        self.frames.last_mut().expect("a frame")
    }

    pub(crate) fn panic(&self, message: impl Into<String>, span: Span) -> Flow<'a> {
        Flow::Panic(message.into(), span)
    }

    pub(crate) fn text(&self, span: Span) -> &'a str {
        span.text(&self.c.src)
    }

    pub(crate) fn literal_type(&self, span: Span) -> Prim {
        self.c
            .literal_types
            .get(&span.start)
            .copied()
            .unwrap_or(Prim::Int)
    }

    /// Calls a function or method. `args` are (name, value) in call order.
    pub(crate) fn call_fn(
        &mut self,
        f: &'a FnDecl,
        self_value: Option<Value<'a>>,
        args: Vec<(Option<&str>, Value<'a>)>,
        span: Span,
    ) -> Eval<'a> {
        self.step(span)?;
        // C functions (spec 10.7) run in native code only.
        if let Some(lib) = &f.c_library {
            return Err(self.panic(
                format!(
                    "`{}` is a C function (library \"{lib}\"): it runs in native code, so run the program with `kavach run --native` or `kavach build` (spec 10.7)",
                    f.name.name
                ),
                span,
            ));
        }
        if crate::stack::too_deep() {
            return Err(self.panic(
                "stack overflow: calls are nested too deeply (spec 3.8)",
                span,
            ));
        }
        let generics = self.next_generics.take().unwrap_or_default();
        self.frames.push(Frame {
            vars: HashMap::new(),
            self_value,
            its: Vec::new(),
            spawned: Vec::new(),
            generics,
        });
        let scoped = f
            .fs_scope
            .as_deref()
            .map(kavach_runtime_core::fs_scope::resolve);
        let scoped = scoped.map(|s| self.fs_scopes.push(s)).is_some();
        let result = self.call_in_frame(f, args, span);
        let result = self.join_spawned(result, span);
        if scoped {
            self.fs_scopes.pop();
        }
        self.frames.pop();
        match result {
            Err(e @ (Flow::Panic(..) | Flow::Exit(_))) => Err(e),
            // Its parameters end with the call (spec 6.1).
            result => {
                self.run_drops(span)?;
                result
            }
        }
    }

    fn call_in_frame(
        &mut self,
        f: &'a FnDecl,
        args: Vec<(Option<&str>, Value<'a>)>,
        span: Span,
    ) -> Eval<'a> {
        let params: Vec<&Param> = f.params.iter().filter(|p| p.name.name != "self").collect();
        let mut values: Vec<Option<Value<'a>>> = vec![None; params.len()];
        let mut next = 0;
        for (name, value) in args {
            let index = match name {
                Some(n) => params.iter().position(|p| p.name.name == n),
                None => {
                    next += 1;
                    Some(next - 1)
                }
            };
            if let Some(slot) = index.and_then(|i| values.get_mut(i)) {
                *slot = Some(value);
            }
        }
        for (p, value) in params.iter().zip(values) {
            let value = match (value, &p.default) {
                (Some(v), _) => v,
                (None, Some(d)) => self.eval(d)?,
                (None, None) => {
                    return Err(self.panic(format!("missing argument `{}`", p.name.name), span));
                }
            };
            if let Some(&def) = self.decl.get(&p.name.span.start) {
                self.frame().vars.insert(def, value);
            }
        }
        for r in &f.requires {
            self.contract("require", r)?;
        }
        let value = match &f.body {
            Some(body) => match self.block(body) {
                Ok(v) => v,
                Err(Flow::Return(v)) => v,
                Err(other) => return Err(other),
            },
            None => Value::Unit,
        };
        if !f.ensures.is_empty() {
            self.result.push(value.clone());
            let checked = f
                .ensures
                .iter()
                .try_for_each(|e| self.contract("ensure", e).map(|_| ()));
            self.result.pop();
            checked?;
        }
        // What the `inout` parameters hold now goes back (spec 3.9).
        let mut out = Vec::new();
        for (i, p) in params.iter().enumerate().filter(|(_, p)| p.is_inout) {
            let Some(&def) = self.decl.get(&p.name.span.start) else {
                continue;
            };
            if let Some(v) = self.frame().vars.get(&def).cloned() {
                out.push((p.name.name.as_str(), i, v));
            }
        }
        self.inout_out = out;
        Ok(value)
    }

    /// Runs `main`, or the top-level statements (spec 1.3).
    pub(crate) fn run_main(&mut self) -> Result<(), Flow<'a>> {
        let main = self
            .fns
            .iter()
            .find(|(_, f)| f.name.name == "main")
            .map(|(_, f)| *f);
        if let Some(main) = main {
            return self
                .call_fn(main, None, Vec::new(), main.name.span)
                .map(|_| ());
        }
        let mut last = None;
        for item in &self.c.file.items {
            if let ItemKind::Stmt(s) = &item.kind {
                self.stmt(s)?;
                last = Some(s.span);
            }
        }
        // Tasks spawned by top-level code are waited for at its end.
        match last {
            Some(span) => self.join_spawned(Ok(Value::Unit), span).map(|_| ()),
            None => Ok(()),
        }
    }

    /// Runs every `example` (spec 7.6) and `test` block (11.8). Returns the
    /// number of failures.
    pub(crate) fn run_tests(&mut self, seed: u64) -> usize {
        let mut failed = 0;
        let mut passed = 0;
        for item in self.c.own_items() {
            let cases: Vec<(String, TestCase<'a>)> = match &item.kind {
                ItemKind::Fn(f) => f
                    .examples
                    .iter()
                    .map(|x| {
                        let fails = x
                            .fails
                            .as_ref()
                            .map(|e| format!(" fails {}", self.text(e.span)))
                            .unwrap_or_default();
                        let label =
                            format!("{}: example {}{fails}", f.name.name, self.text(x.expr.span));
                        let run: TestCase<'a> = Box::new(move |i: &mut Self| match &x.fails {
                            None => Ok(i.eval(&x.expr)? == Value::Bool(true)),
                            Some(want) => match i.eval(&x.expr) {
                                Err(Flow::Fail(got)) => Ok(got == i.eval(want)?),
                                Err(other) => Err(other),
                                Ok(_) => Ok(false),
                            },
                        });
                        (label, run)
                    })
                    .collect(),
                ItemKind::Test(t) => {
                    let label = format!("test {}", self.text(t.name.span));
                    let run: TestCase<'a> =
                        Box::new(move |i: &mut Self| i.block(&t.body).map(|_| true));
                    vec![(label, run)]
                }
                _ => Vec::new(),
            };
            for (label, run) in cases {
                // Each test's own numbers, whatever runs before it.
                let salt = crate::testing::values::Rng::hash(seed, &label);
                self.random = Some(crate::testing::values::Rng::new(salt));
                self.frozen = None;
                let outcome = run(self);
                match outcome {
                    Ok(true) => {
                        passed += 1;
                        self.out.push_str(&format!("✓ {label}\n"));
                    }
                    Ok(false) => {
                        failed += 1;
                        self.out.push_str(&format!("✗ {label}\n"));
                    }
                    Err(flow) => {
                        failed += 1;
                        let why = match flow {
                            Flow::Panic(m, _) => format!("panic: {m}"),
                            Flow::Fail(e) => format!("failed with error {}", e.display()),
                            _ => "stopped early".to_string(),
                        };
                        self.out.push_str(&format!("✗ {label}\n    {why}\n"));
                    }
                }
            }
        }
        self.out
            .push_str(&format!("{passed} passed, {failed} failed\n"));
        failed
    }

    pub(crate) fn str_value(&self, text: &str) -> Value<'a> {
        Value::Str(Rc::from(text))
    }
}
