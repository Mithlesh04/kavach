//! `kavach test --keep` (spec 11.4): a found case that now passes becomes
//! an `example` clause on its function, so the fixed bug is documented
//! where people read the function. The example says what the call gives
//! now: `example f(x) == result`, or `example f(x) fails error`.

use super::TestOptions;
use super::found::{can_save, write_case};
use crate::interp::Interp;
use crate::value::Flow;

/// What `--keep` found: examples to add, the cases that stay found, and
/// why some stay.
#[derive(Debug, Default)]
pub struct Kept {
    /// The start of the function's name, and the clause to add to it.
    pub examples: Vec<(u32, String)>,
    /// Found cases that stay in the file.
    pub remaining: Vec<String>,
    pub notes: Vec<String>,
}

impl<'a> Interp<'a> {
    pub(crate) fn keep_found(&mut self, opts: &TestOptions) -> Kept {
        let (targets, _) = self.targets();
        let mut kept = Kept::default();
        // The file the command names: its found cases (`opts.found_file`).
        let saved = (opts.found_file.as_deref()).map_or_else(Vec::new, Self::saved_cases);
        for line in saved {
            let target = targets
                .iter()
                .find(|t| line.starts_with(&format!("{}(", t.name)));
            let Some((t, args)) = target.and_then(|t| Some((t, self.read_case(t, &line)?))) else {
                kept.notes
                    .push(format!("{line}: no function takes this case any more"));
                kept.remaining.push(line);
                continue;
            };
            let call = if t.has_self {
                let method = t.name.rsplit('.').next().unwrap_or_default();
                let rest: Vec<String> = args[1..].iter().map(|a| self.code_of(a)).collect();
                format!("{}.{method}({})", self.code_of(&args[0]), rest.join(", "))
            } else {
                write_case(self, &t.name, &args)
            };
            let clause = match self.call_case(t, &args, opts.step_budget) {
                Ok(_) if t.f.ret.is_none() => {
                    Err("it returns nothing, and an example needs a value".to_string())
                }
                Ok(v) if can_save(&v) => Ok(format!("example {call} == {}", self.code_of(&v))),
                Err(Flow::Fail(e)) if can_save(&e) => {
                    Ok(format!("example {call} fails {}", self.code_of(&e)))
                }
                Err(Flow::Panic(_, span)) if t.f.requires.iter().any(|r| r.span == span) => {
                    Err("it no longer meets the function's `require`".into())
                }
                Err(Flow::Panic(m, _)) => {
                    Err(format!("it still fails ({m}): fix the function first"))
                }
                _ => Err("its result can't be written as code".to_string()),
            };
            match clause {
                Ok(c) => kept.examples.push((t.f.name.span.start, c)),
                Err(why) => {
                    kept.notes.push(format!("{line}: {why}"));
                    kept.remaining.push(line);
                }
            }
        }
        kept
    }
}
