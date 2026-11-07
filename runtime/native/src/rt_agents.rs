//! Agents in native code (spec 8.3): the guards of started agents, the
//! stack of running agents, and the checks of each capability call. The
//! bookkeeping is `kavach_runtime_core::agents`, the interpreter's too, so
//! both decide and log alike. The code that calls these ops is made by
//! `compiler/kir/src/lower_agents.rs` and `lower_guard.rs`.

use crate::display::{decimal_of, display};
use crate::heap::{Data, alloc, obj, release, retain, slots};
use crate::rt::type_of;
use crate::shapes::{new_text, shape, with_str};
use kavach_kir::{RtOp, Shape, Type};
use kavach_runtime_core::agents::{Check, Guard, Mode, Num, Rule, Settings, Step};
use kavach_runtime_core::json::{self, Tree};
use std::cell::RefCell;
use std::sync::Mutex;

/// A started agent: its guard, and from its description, every capability
/// method's name and parameters (secret or not), and each approval's
/// condition as written.
struct Started {
    guard: Guard,
    calls: Vec<(String, Vec<(String, bool)>)>,
    approvals: Vec<String>,
}

/// A call being checked: its agent, and while a rule is checked, whether
/// the rule used `sum` or `count` and what `sum` added.
struct Pending {
    agent: usize,
    check: Check,
    used: bool,
    added: Option<Num>,
    /// The next `approve`'s `within`, in milliseconds (`AgentWithin`).
    within: Option<i64>,
}

static STARTED: Mutex<Vec<Started>> = Mutex::new(Vec::new());
static PENDING: Mutex<Vec<Option<Pending>>> = Mutex::new(Vec::new());

/// A running agent: its record, its kind, and the journaled event its
/// handler is in (spec 8.5), 0 for none.
pub(crate) type Inside = (i64, i64, i64);

thread_local! {
    /// The running agents, innermost last.
    pub(crate) static RUNNING: RefCell<Vec<Inside>> = const { RefCell::new(Vec::new()) };
    /// How many capability calls are running: their implementations'
    /// clock reads and waits aren't journaled, the call's outcome is.
    static IN_CALL: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The journal handle and name of the agent whose capability call runs
/// now, while its implementation runs.
pub(crate) fn call_running() -> Option<(i64, String)> {
    if IN_CALL.with(|n| n.get()) == 0 {
        return None;
    }
    let handle = RUNNING.with(|r| r.borrow().last().map(|(p, ..)| slots(*p)[0] as i64))?;
    let name = with_agent(handle as usize, |s| s.guard.settings.name.clone());
    Some((handle, name))
}

/// The journal handle of the agent whose handler runs now, when no
/// capability's implementation is running inside it.
pub(crate) fn handler_running() -> Option<i64> {
    if IN_CALL.with(|n| n.get()) > 0 {
        return None;
    }
    RUNNING.with(|r| r.borrow().last().map(|(p, ..)| slots(*p)[0] as i64))
}

/// The journaled event the running handler is in, if any: its calls go
/// to the journal under it, whichever task makes them.
pub(crate) fn event() -> Option<i64> {
    RUNNING
        .with(|r| r.borrow().last().map(|(.., e)| *e))
        .filter(|e| *e != 0)
}

/// The running handler's event has begun: number `event`.
pub(crate) fn set_event(event: i64) {
    RUNNING.with(|r| {
        if let Some(top) = r.borrow_mut().last_mut() {
            top.2 = event;
        }
    });
}

/// The agent ops (operands owned). `code` is the result type.
pub fn agent_op(op: RtOp, a: i64, b: i64, c: i64, code: i64, line: i32) -> i64 {
    crate::rt_tasks::turn();
    match op {
        RtOp::AgentStart => {
            let description = with_str(a, |s| s.to_string());
            release(a);
            let agent = start(&description);
            // One program per agent (decision 0073): its folder is locked.
            if let Some(state) = &agent.guard.settings.state
                && let Err(why) = kavach_runtime_core::agents::lock_folder(state)
            {
                crate::rt::panic_with(&why, line);
            }
            let mut started = STARTED.lock().unwrap_or_else(|e| e.into_inner());
            started.push(agent);
            (started.len() - 1) as i64
        }
        RtOp::AgentPush => {
            let handle = slots(a)[0] as usize;
            with_agent(handle, |s| s.guard.next_request());
            RUNNING.with(|r| r.borrow_mut().push((a, b, 0)));
            0
        }
        RtOp::AgentPop => {
            if let Some((p, ..)) = RUNNING.with(|r| r.borrow_mut().pop()) {
                release(p);
            }
            0
        }
        RtOp::AgentCurrent => match RUNNING.with(|r| r.borrow().last().copied()) {
            Some((p, kind, _)) if kind == a => {
                retain(p);
                p
            }
            _ => 0,
        },
        RtOp::AgentBegin => {
            IN_CALL.with(|n| n.set(n.get() + 1));
            let handle = a as usize;
            let (call, shown) = with_agent(handle, |s| {
                let (call, params) = &s.calls[b as usize];
                (call.clone(), shown(call, params, c))
            });
            release(c);
            let check = with_agent(handle, |s| s.guard.begin(&call, &shown));
            let pending = Pending {
                agent: handle,
                check,
                used: false,
                added: None,
                within: None,
            };
            let mut all = PENDING.lock().unwrap_or_else(|e| e.into_inner());
            match all.iter().position(Option::is_none) {
                Some(i) => {
                    all[i] = Some(pending);
                    i as i64
                }
                None => {
                    all.push(Some(pending));
                    (all.len() - 1) as i64
                }
            }
        }
        RtOp::AgentCount => with_pending(a, |p, s| {
            p.used = true;
            s.guard.totals(b as usize).1 + 1
        }),
        RtOp::AgentTokens => with_pending(a, |p, s| {
            p.used = true;
            p.check.tokens.push(b as usize);
            match s.guard.totals(b as usize).0 {
                Some(kavach_runtime_core::agents::Num::Int(n)) => n,
                _ => 0,
            }
        }),
        RtOp::AgentSum => {
            let ty = type_of(code);
            let x = num_of(c, ty);
            let total = with_pending(a, |p, s| {
                p.used = true;
                p.added = Some(x.clone());
                match s.guard.totals(b as usize).0 {
                    Some(before) => before.add(&x),
                    None => Some(x.clone()),
                }
            });
            let Some(total) = total else {
                crate::rt::panic_with("a limit's total doesn't fit", 0);
            };
            slot_of(&total, ty)
        }
        RtOp::AgentLimit => with_pending(a, |p, s| {
            let (used, added) = (p.used, p.added.take());
            p.used = false;
            i64::from(s.guard.limit(&mut p.check, b as usize, c != 0, used, added))
        }),
        RtOp::AgentNeed => {
            let by = with_str(c, |t| t.to_string());
            release(c);
            with_pending(a, |p, s| {
                let when = s.approvals.get(b as usize).cloned().unwrap_or_default();
                let within = p.within.take();
                s.guard.need(&mut p.check, &by, &when, within);
                0
            })
        }
        // `within d` of the `approve` whose condition holds: `b` is `d` in
        // milliseconds.
        RtOp::AgentApprovals | RtOp::AgentAnswer => crate::rt_approvals::run(op, a, b, code),
        RtOp::AgentReplay
        | RtOp::AgentReplayWait
        | RtOp::AgentReplayEnd
        | RtOp::AgentJoining
        | RtOp::AgentJoined => crate::rt_replays::run(op, a, b, c, code, line),
        RtOp::AgentWithin => with_pending(a, |p, _| {
            p.within = Some(b);
            0
        }),
        // The last check: a call that passes starts now, and counts from
        // now, so a crash while it runs can't leave it uncounted (decision
        // 0062).
        // A request a person answers later (decision 0072) is looked at
        // each second, with the locks let go meanwhile, so other tasks
        // and agents go on.
        RtOp::AgentDecide => loop {
            let handle = with_pending(a, |p, _| p.agent) as i64;
            let name = with_agent(handle as usize, |s| s.guard.settings.name.clone());
            let key = crate::rt_journal::call_key_of(handle, &name);
            let step = with_pending(a, |p, s| s.guard.step(&mut p.check, key.as_deref()));
            match step {
                Step::Done(refused) => {
                    crate::rt_replays::asking(false);
                    if !refused {
                        with_pending(a, |p, s| s.guard.start(&mut p.check));
                    }
                    return i64::from(refused);
                }
                Step::Wait => {
                    // A replayed event that gets here lets `start` return.
                    crate::rt_replays::asking(true);
                    let next = std::time::Instant::now() + std::time::Duration::from_secs(1);
                    crate::rt_waits::sleep_until(next, line);
                    // Cancelled (the program ends): the handler returns at
                    // once, its event unfinished (`kir/src/cancel.rs`).
                    if crate::rt_tasks::CANCELLING.get() {
                        crate::rt_replays::asking(false);
                        return 0;
                    }
                }
            }
        },
        RtOp::AgentReason => {
            let why = with_pending(a, |p, _| p.check.refused.clone().unwrap_or_default());
            new_text(&why, type_of(code))
        }
        RtOp::AgentDone => {
            IN_CALL.with(|n| n.set(n.get().saturating_sub(1)));
            let pending = PENDING.lock().unwrap_or_else(|e| e.into_inner())[a as usize].take();
            let Some(p) = pending else {
                return 0;
            };
            let result = match b {
                0 => format!("denied: {}", p.check.refused.clone().unwrap_or_default()),
                1 => match first(c) {
                    None => "ok".to_string(),
                    Some((v, t)) => format!("ok {}", display(v, t, true)),
                },
                _ => match first(c) {
                    Some((e, t)) => format!("error {}", display(e, t, true)),
                    None => "error".to_string(),
                },
            };
            if c != 0 {
                release(c);
            }
            let mut p = p;
            if p.check.call == "Ai.ask"
                && let Some((shown, took)) = crate::rt_ai::take_last()
            {
                p.check.model = Some(shown);
                p.check.took(took);
            }
            with_agent(p.agent, |s| {
                match b {
                    1 => s.guard.record(&p.check),
                    2 => s.guard.unrecord(&p.check),
                    _ => {}
                }
                s.guard.audit(&p.check, &result);
            });
            0
        }
        _ => 0,
    }
}

fn with_agent<R>(handle: usize, f: impl FnOnce(&mut Started) -> R) -> R {
    let mut started = STARTED.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut started[handle])
}

fn with_pending<R>(id: i64, f: impl FnOnce(&mut Pending, &mut Started) -> R) -> R {
    let mut all = PENDING.lock().unwrap_or_else(|e| e.into_inner());
    let p = all[id as usize].as_mut().expect("a call being checked");
    let mut started = STARTED.lock().unwrap_or_else(|e| e.into_inner());
    f(p, &mut started[p.agent])
}

/// A guard from the description the compiler wrote (`lower_agents.rs`).
fn start(description: &str) -> Started {
    let tree = json::parse(description, &json::Limits::default()).unwrap_or(Tree::Null);
    let field = |name: &str| match &tree {
        Tree::Object(fs) => fs.iter().find(|(k, _)| k == name).map(|(_, v)| v),
        _ => None,
    };
    let text = |t: Option<&Tree>| match t {
        Some(Tree::Text(s)) => Some(s.clone()),
        _ => None,
    };
    let list = |t: Option<&Tree>| match t {
        Some(Tree::Array(items)) => items.clone(),
        _ => Vec::new(),
    };
    let mode = match text(field("mode")).as_deref() {
        Some("observe") => Mode::Observe,
        Some("suggest") => Mode::Suggest,
        _ => Mode::Enforce,
    };
    let settings = Settings {
        name: text(field("name")).unwrap_or_default(),
        mode,
        audit: text(field("audit")).map(Into::into),
        audit_json: matches!(field("audit_json"), Some(Tree::Bool(true))),
        otlp: match field("otlp") {
            Some(Tree::Array(parts)) => match &parts[..] {
                [Tree::Text(endpoint), Tree::Text(service)] => {
                    kavach_runtime_core::agents::set_audit_sink(
                        kavach_runtime_net::otlp::export_audit,
                    );
                    crate::rt::at_exit(kavach_runtime_net::otlp::finish);
                    Some((endpoint.clone(), service.clone()))
                }
                _ => None,
            },
            _ => None,
        },
        state: text(field("state")).map(Into::into),
    };
    let rules = list(field("rules"))
        .iter()
        .map(|r| {
            let parts = list(Some(r));
            let target = text(parts.first()).unwrap_or_default();
            let expr = text(parts.get(1)).unwrap_or_default();
            Rule::new(&target, &expr, text(parts.get(2)).as_deref())
        })
        .collect();
    let approvals = list(field("approvals"))
        .iter()
        .map(|a| text(Some(a)).unwrap_or_default())
        .collect();
    let calls = list(field("calls"))
        .iter()
        .map(|c| {
            let parts = list(Some(c));
            let params = list(parts.get(1))
                .iter()
                .map(|p| {
                    let pair = list(Some(p));
                    let secret = matches!(pair.get(1), Some(Tree::Bool(true)));
                    (text(pair.first()).unwrap_or_default(), secret)
                })
                .collect();
            (text(parts.first()).unwrap_or_default(), params)
        })
        .collect();
    Started {
        guard: Guard::new(settings, rules),
        calls,
        approvals,
    }
}

/// `Payments.refund(order_id: 1042, amount: 1499)`, from the arguments in
/// tuple `args` (borrowed); secret ones are written `<secret>` (spec 8.7).
fn shown(call: &str, params: &[(String, bool)], args: i64) -> String {
    if call == "Ai.ask" {
        // The prompt's start, as the interpreter writes it.
        let prompt = with_str(slots(args)[0] as i64, |s| {
            s.chars().take(60).collect::<String>()
        });
        return format!("ask(\"{prompt}\")");
    }
    let types = match shape(obj(args).ty()) {
        Shape::Tuple(ts) => ts.clone(),
        _ => Vec::new(),
    };
    let values = slots(args);
    let shown: Vec<String> = params
        .iter()
        .zip(types.iter().zip(values))
        .map(|((name, secret), (t, v))| match secret {
            true => format!("{name}: <secret>"),
            false => format!("{name}: {}", display(*v as i64, *t, false)),
        })
        .collect();
    format!("{call}({})", shown.join(", "))
}

/// The value in one-item tuple `t` (borrowed) and its type; `None` for
/// nothing (`Unit`).
fn first(t: i64) -> Option<(i64, Type)> {
    if t == 0 {
        return None;
    }
    let ty = match shape(obj(t).ty()) {
        Shape::Tuple(ts) => ts.first().copied()?,
        _ => return None,
    };
    (ty != Type::Unit).then(|| (slots(t)[0] as i64, ty))
}

/// The number in slot `x` (owned) of type `ty`.
fn num_of(x: i64, ty: Type) -> Num {
    match ty {
        Type::F64 => Num::Float(f64::from_bits(x as u64)),
        Type::F32 => Num::Float(f64::from(crate::display::f32_of(x))),
        Type::Ref(_) => {
            let d = decimal_of(x);
            release(x);
            Num::Decimal(d)
        }
        _ => Num::Int(x),
    }
}

/// A slot of type `ty` holding `n`.
fn slot_of(n: &Num, ty: Type) -> i64 {
    match n {
        Num::Int(i) => *i,
        Num::Float(f) => f.to_bits() as i64,
        Num::Decimal(d) => alloc(ty, Data::Decimal(Box::new(*d))),
    }
}

/// A new task runs inside the agent that spawned it (spec 8.3): the
/// running agent, with a reference for the task.
/// Puts `agents` in place of this thread's running agents, giving those:
/// tasks switching on one thread keep their own (`rt_coroutines.rs`).
pub(crate) fn swap_agents(agents: Vec<Inside>) -> Vec<Inside> {
    RUNNING.with(|r| std::mem::replace(&mut *r.borrow_mut(), agents))
}

pub(crate) fn inherited() -> Option<Inside> {
    let top = RUNNING.with(|r| r.borrow().last().copied());
    if let Some((p, ..)) = top {
        retain(p);
    }
    top
}

/// The task's thread starts inside `agent`.
pub(crate) fn inherit(agent: Option<Inside>) {
    if let Some(a) = agent {
        RUNNING.with(|r| r.borrow_mut().push(a));
    }
}

/// The task has ended: its agents are let go of.
pub(crate) fn leave() {
    while let Some((p, ..)) = RUNNING.with(|r| r.borrow_mut().pop()) {
        release(p);
    }
}

// The group's runtime entry (`RtOp::entry`, decision 0024).
crate::rt_data::entry!(kv_rt_agents, |op, a, b, c, code, line| {
    match op {
        RtOp::AgentRun => crate::rt_events::run(a, b, c, line),
        _ => agent_op(op, a, b, c, code, line),
    }
});

/// The events agent `handle`'s journal set aside when it was opened, in
/// its audit log (decision 0073).
/// A line `text` in agent `handle`'s audit log.
pub(crate) fn note(handle: i64, text: &str) {
    with_agent(handle as usize, |s| {
        let _ = s.guard.note(text);
    });
}

pub(crate) fn note_set_aside(handle: i64, events: &[(i64, String)]) {
    if !events.is_empty() {
        with_agent(handle as usize, |s| s.guard.set_aside(events));
    }
}
