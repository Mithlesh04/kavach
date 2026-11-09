//! A `tokens` limit in native code (spec 8.3): what `tokens` reads, with
//! the `ask`s still waiting for their answer counted as the agent's
//! settings say; a rule's verdict, which under `UnderWay::Average` can be
//! "wait for the `ask`s under way, then check again"; and the most an
//! `ask` can take. The bookkeeping is `kavach_runtime_core::agents`, the
//! interpreter's too.

use crate::rt_agents::with_pending;
use kavach_kir::RtOp;
use kavach_runtime_core::agents::largest_ask;

/// How often an `ask` waiting for those under way looks again.
const LOOK: std::time::Duration = std::time::Duration::from_millis(20);

/// The verdict that checks the rule again (`compiler/kir/src/lower_guard.rs`).
const AGAIN: i64 = 2;

/// `AgentTokens`, `AgentLimit` and `AgentBound`, for the call being
/// checked `a` and its rule `b` (for `AgentBound`, `b` is `max_tokens`).
pub(crate) fn run(op: RtOp, a: i64, b: i64, c: i64, line: i32) -> i64 {
    let rule = b as usize;
    match op {
        RtOp::AgentTokens => with_pending(a, |p, s| {
            p.used = true;
            if !p.check.tokens.contains(&rule) {
                p.check.tokens.push(rule);
            }
            s.guard.tokens(rule, &p.check)
        }),
        RtOp::AgentBound => with_pending(a, |p, _| {
            p.check.bound = largest_ask(p.prompt_bytes, b);
            0
        }),
        RtOp::AgentLimit => {
            let verdict = with_pending(a, |p, s| {
                let ok = c != 0;
                if !ok && s.guard.waits_for_asks(&p.check, rule) {
                    p.used = false;
                    p.added = None;
                    return None;
                }
                let (used, added) = (p.used, p.added.take());
                p.used = false;
                Some(i64::from(s.guard.limit(
                    &mut p.check,
                    rule,
                    ok,
                    used,
                    added,
                )))
            });
            if let Some(v) = verdict {
                return v;
            }
            // The locks are let go while it waits, so the `ask`s under
            // way can answer.
            crate::rt_waits::sleep_until(std::time::Instant::now() + LOOK, line);
            // Cancelled (the program ends): refused, so the handler ends.
            if crate::rt_tasks::CANCELLING.get() {
                return 1;
            }
            AGAIN
        }
        _ => 0,
    }
}
