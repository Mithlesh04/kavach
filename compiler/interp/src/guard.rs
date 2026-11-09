//! The guard on every capability call an agent's handler makes (spec 8.3,
//! "The order of the checks"): its limits, with totals over rolling
//! windows, then its approvals, as the agent's mode says (8.5). The
//! bookkeeping (totals, the state file, asking a person, the audit log of
//! 8.6) is `kavach_runtime_core::agents`, which native code uses too;
//! here the rules are evaluated.

use crate::agents::Running;
use crate::interp::{Frame, Interp};
use crate::value::{Flow, Value};
use kavach_ast::{AgentMember, Expr, Path, Span};
use kavach_checker::Prim;
use kavach_resolver::{DefId, DefKind};
use kavach_runtime_core::agents::{Check, Num, Rule, Step, covers};

/// How often an `ask` waiting for those under way looks again (spec 8.3).
const UNDER_WAY_LOOK: std::time::Duration = std::time::Duration::from_millis(20);

/// The code of one limit rule: its expression and the parameters it names.
#[derive(Debug)]
pub(crate) struct RuleCode<'a> {
    expr: &'a Expr,
    params: Vec<(DefId, String)>,
}

/// One `approve … when … by … within …`.
#[derive(Debug)]
pub(crate) struct Approval<'a> {
    target: String,
    when: &'a Expr,
    by: &'a Expr,
    within: Option<&'a Expr>,
    params: Vec<(DefId, String)>,
}

/// While a limit rule is checked: the totals before this call, and what
/// `sum(…)` adds for it (spec 8.3).
#[derive(Debug, Default)]
pub(crate) struct Totals<'a> {
    pub(crate) sum: Option<Value<'a>>,
    pub(crate) count: i128,
    pub(crate) added: Option<Value<'a>>,
    /// Whether the rule used `sum` or `count`: only then are its calls
    /// kept (a rule on single calls needs no history).
    pub(crate) used: bool,
    /// Whether it read `tokens`: then each call adds what it took.
    pub(crate) tokens: bool,
    /// What `tokens` reads: the window's `ask`s, with those under way
    /// counted as the settings say (`Guard::tokens`).
    pub(crate) tokens_now: i128,
}

fn target_of(path: &Path) -> String {
    let names: Vec<&str> = path.segments.iter().map(|s| s.name.as_str()).collect();
    names.join(".")
}

fn value_of<'a>(n: Num) -> Value<'a> {
    match n {
        Num::Int(i) => Value::Int(i128::from(i), Prim::Int),
        Num::Float(f) => Value::Float(f),
        Num::Decimal(d) => Value::Decimal(d),
    }
}

fn num_of(v: &Value<'_>) -> Option<Num> {
    match v {
        Value::Int(n, _) => i64::try_from(*n).ok().map(Num::Int),
        Value::Float(f) => Some(Num::Float(*f)),
        Value::Decimal(d) => Some(Num::Decimal(*d)),
        _ => None,
    }
}

impl<'a> Interp<'a> {
    /// The parameters used in `e`: the definitions the resolver gave the
    /// limited method's parameters inside a `limit` or `approve`.
    fn rule_params(&self, e: &Expr) -> Vec<(DefId, String)> {
        let mut out: Vec<(DefId, String)> = Vec::new();
        for (&start, &def) in &self.c.resolution.uses {
            let d = &self.c.resolution.defs[def];
            let inside = e.span.start <= start && start < e.span.end;
            let param = d.kind == DefKind::Param && d.span.is_none();
            if inside && param && !out.iter().any(|(x, _)| *x == def) {
                out.push((def, d.name.clone()));
            }
        }
        out
    }

    /// The rules (for the guard, and their code) and the approvals of an
    /// agent's declaration.
    #[allow(clippy::type_complexity)]
    pub(crate) fn guards(
        &self,
        members: &'a [AgentMember],
    ) -> (Vec<Rule>, Vec<RuleCode<'a>>, Vec<Approval<'a>>) {
        let (mut rules, mut code, mut approvals) = (Vec::new(), Vec::new(), Vec::new());
        for m in members {
            match m {
                AgentMember::Limit { target, rules: rs } => {
                    for r in rs {
                        let per = r.per.as_ref().map(|p| p.name.as_str());
                        rules.push(Rule::new(&target_of(target), &self.source_of(&r.expr), per));
                        code.push(RuleCode {
                            expr: &r.expr,
                            params: self.rule_params(&r.expr),
                        });
                    }
                }
                AgentMember::Approve {
                    target,
                    when,
                    by,
                    within,
                } => approvals.push(Approval {
                    target: target_of(target),
                    when,
                    by,
                    within: within.as_ref(),
                    params: self.rule_params(when),
                }),
                _ => {}
            }
        }
        (rules, code, approvals)
    }

    /// Evaluates `e` with the call's arguments bound to `params`.
    fn with_args(
        &mut self,
        e: &'a Expr,
        params: &[(DefId, String)],
        args: &[(String, Value<'a>)],
    ) -> Result<Value<'a>, Flow<'a>> {
        let mut vars = std::collections::HashMap::new();
        for (def, name) in params {
            if let Some((_, v)) = args.iter().find(|(n, _)| n == name) {
                vars.insert(*def, v.clone());
            }
        }
        self.frames.push(Frame {
            vars,
            self_value: None,
            its: Vec::new(),
            spawned: Vec::new(),
            generics: std::collections::HashMap::new(),
        });
        let v = self.eval(e);
        self.frames.pop();
        v
    }

    /// The text of `e` as written.
    fn source_of(&self, e: &Expr) -> String {
        self.c.src[e.span.start as usize..e.span.end as usize].to_string()
    }

    /// Steps 3 and 4 for one call (`call` is `Cap.method`, `shown` the call
    /// as the audit log writes it, `bound` the most an `ask` can take).
    pub(crate) fn check_call(
        &mut self,
        agent: &Running<'a>,
        (call, shown, bound): (&str, &str, i64),
        args: &[(String, Value<'a>)],
        span: Span,
    ) -> Result<Check, Flow<'a>> {
        let mut c = agent.guard.borrow().begin(call, shown);
        c.bound = bound;
        let covering = agent.guard.borrow().rules_for(call);
        for i in covering {
            loop {
                let (sum, count) = agent.guard.borrow_mut().totals(i);
                let tokens_now = agent.guard.borrow_mut().tokens(i, &c);
                self.totals = Some(Totals {
                    sum: sum.map(value_of),
                    count: i128::from(count),
                    added: None,
                    used: false,
                    tokens: false,
                    tokens_now: i128::from(tokens_now),
                });
                let code = &agent.rules[i];
                let ok = self.with_args(code.expr, &code.params, args);
                let totals = self.totals.take().unwrap_or_default();
                let ok = ok? == Value::Bool(true);
                let added = totals.added.as_ref().and_then(num_of);
                if totals.tokens && !c.tokens.contains(&i) {
                    c.tokens.push(i);
                }
                // `ask`s under way that may leave room: wait for their
                // answers, then check again (spec 8.3).
                if !ok && agent.guard.borrow().waits_for_asks(&c, i) {
                    let next = std::time::Instant::now() + UNDER_WAY_LOOK;
                    self.sleep_until(next, span)?;
                    continue;
                }
                if agent
                    .guard
                    .borrow()
                    .limit(&mut c, i, ok, totals.used, added)
                {
                    return Ok(c);
                }
                break;
            }
        }
        // Step 4: who must approve.
        for a in agent.approvals.iter().filter(|a| covers(&a.target, call)) {
            if self.with_args(a.when, &a.params, args)? == Value::Bool(true) {
                let by = self.eval(a.by)?.display();
                // `within d`: the `Duration`'s `ms`.
                let within = match a.within {
                    Some(w) => match self.eval(w)? {
                        Value::Record { fields, .. } => match fields.first().map(|f| &f.value) {
                            Some(Value::Int(ms, _)) => i64::try_from(*ms).ok(),
                            _ => None,
                        },
                        _ => None,
                    },
                    None => None,
                };
                agent
                    .guard
                    .borrow()
                    .need(&mut c, &by, &self.source_of(a.when), within);
                break;
            }
        }
        // A request a person answers later (decision 0072): looked at each
        // second, while other tasks run.
        let name = &agent.decl.name.name;
        let key = (agent.journal.as_ref())
            .zip(self.events.last())
            .and_then(|(j, e)| j.borrow().call_key(*e, name));
        let mut asked = false;
        while agent.guard.borrow().step(&mut c, key.as_deref()) == Step::Wait {
            // A replayed event that gets here lets `start` return (0072).
            if !asked {
                agent.asking.borrow_mut().push(self.task);
                asked = true;
            }
            let next = std::time::Instant::now() + std::time::Duration::from_secs(1);
            let slept = self.sleep_until(next, span);
            if slept.is_err() {
                agent.asking.borrow_mut().retain(|t| *t != self.task);
            }
            slept?;
        }
        if asked {
            agent.asking.borrow_mut().retain(|t| *t != self.task);
        }
        Ok(c)
    }
}
