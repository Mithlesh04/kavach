//! The two ways to run a file: `main` (or the top-level statements) and
//! its tests (`example` clauses and `test` blocks).

use crate::interp::Interp;
use crate::value::{Flow, Value};
use kavach_ast::ItemKind;

/// One `example` or `test`: true when it passes.
type TestCase<'a> = Box<dyn Fn(&mut Interp<'a>) -> Result<bool, Flow<'a>> + 'a>;

impl<'a> Interp<'a> {
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
}
