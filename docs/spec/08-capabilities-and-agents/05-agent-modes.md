# 8.5 Agent modes: earning trust step by step

Each agent runs in one mode, set in its `settings`. A company can adopt Kavach without trusting it on day one.

An agent's settings are an `AgentSettings` value from `std.ai`, written in the agent after its `model` ([decision 0060](../../decisions/0060-agent-settings-are-chosen-in-code.md)). As with `model`, a function with no parameters shares one between agents, and `with { … }` changes fields for one agent (2.4):

```kavach
fn standard() -> AgentSettings {
    AgentSettings(audit: Audit(format: AuditFormat.JsonLines))
}

agent RefundBot {
    model careful()
    settings standard() with { mode: AgentMode.Observe }
    can Payments.refund
    …
}
```

```kavach
type AgentSettings {
    mode?: AgentMode       // none: Enforce; Observe, Suggest or Enforce
    audit?: Audit          // none: no audit log; an `Audit` turns it on (8.6)
    state?: Bool           // none: true in a project; limit totals, kept across restarts
    journal?: Bool         // none: true in a project; handlers resumed after a crash (below)
    folder?: String        // none: `agents/<Name>/` beside `kavach.toml`; where the agent's files go
    asks_under_way?: UnderWay  // none: Average; how `tokens` counts `ask`s waiting for their answer (8.3)
}

type UnderWay = Average | Largest | NotCounted

type Audit {
    file?: String          // none: "audit.log", in the agent's folder
    format?: AuditFormat   // none: Text; JsonLines: one JSON object per entry
    otlp?: Bool            // none: false; true: each entry to the `[log.otlp]` collector too
}
```

- What survives a restart is on unless the settings say otherwise: in a project, every agent keeps its state (`state.json`) and its journal (`journal`) in its folder, `agents/<Name>/` beside `kavach.toml`. The project's `.gitignore` from `kavach new` leaves that folder out. The audit log is off until the settings turn it on.
- `[agents] folder = "…"` in `kavach.toml` moves every agent's folder (`<folder>/<Name>/`), and an agent's own `folder` moves its own; paths are relative to the folder of `kavach.toml`. A program with no `kavach.toml` keeps no state and no journal unless its settings name a `folder`.
- `kavach test` and `kavach run --dry-run` keep no state and no journal, so a test starts fresh every time, and a simulated result is never replayed by a real run.
- The settings are worked out when the program is built, like `model`; a field the record doesn't have is a compile error. `[agents.<Name>]` in `kavach.toml` is an error that says where each of its keys went.

### Durable handlers

A handler can stop half way: the program crashes, or the machine restarts. With its journal on (the default in a project), nothing is lost and nothing happens twice ([decision 0025](../../decisions/0025-agents-resume-by-replaying-a-journal.md)). So a handler must be safe to run again from its start: it reaches the world only through capabilities, whose outcomes the journal replays.

The journal records each event a handler gets, with its arguments, then the outcome of each capability call it makes (and each `ask`), then that the handler returned. When the agent starts again (`RefundBot.start(…)`), every event that never returned runs again from the beginning, before `start` returns. A call already in the journal gives its recorded outcome without running, so a refund is not paid twice; the first call that isn't there runs for real, and so does the rest. This works because a handler reaches the world only through capabilities: everything else it does is decided by its arguments and those outcomes. If the handler makes a different call than the journal recorded (the program changed), the event stops with a panic. Several events can be under way at once, each in its own task (one waits for a person while another runs): each call goes to the journal under the event that made it, and so does a call from a task spawned in its handler.

Time is replayed too ([decision 0062](../../decisions/0062-crashes-restarts-and-clocks.md)). A durable handler's clock reads (`time.now_ms()`, `today()`, `now()`) are journaled like outcomes, so a run after a crash sees the same time and makes the same calls. A wait (`sleep`, `every`, a timeout) records when it ends, as a moment in UTC, so after a restart it waits only for what is left:

```kavach
on remind(order_id: Int) ! Error {
    try Email.send("Your order {order_id} ships soon.")
    time.sleep(1.days)            // a crash an hour in: after the restart, 23 hours more
    try Email.send("Your order {order_id} has shipped.")
}
```

A call the crash cut off, before its outcome was journaled, runs again: a call runs at least once and may run twice. Each call in a durable handler carries an idempotency key, the same in the run after a crash, so the service it reaches can drop the repeat; its implementation reads it with `agents.call_key()` (14.16). Limits count a call when it starts, so a crash can't leave it uncounted. The journal holds arguments and outcomes as they are, secret ones too, so it needs the same care as other secrets: it is state, not a log. When the agent starts, the events that ended are dropped from it, so it stays small. A journal that can't be written (a full disk) stops the program with a panic, as a database stops when its log fails: going on would break the promise.

The state file holds each limit's calls inside its window, so totals such as `sum(amount) <= 50_000 per day` hold across restarts; the runtime writes it after every call that counts. With `state: false`, totals last only while the program runs. An approval in a durable handler waits across restarts (8.3).

**An event that never finishes is set aside** ([decision 0073](../../decisions/0073-durable-agents-across-restarts.md)). Each start of an unfinished event is written to the journal. An event that started 3 times without returning (each time the program panicked or crashed in it) would only stop the agent again, so the next start moves it out: its lines go from the journal to `set-aside` in the agent's folder, the audit log says `set aside: event #17 of request (started 3 times without finishing)`, and the agent starts with the rest. A person can read the event there, fix the program and send it again. It is what message queues call a dead-letter queue.

**One program per agent.** An agent with a state file or a journal keeps them in its own folder, and the program that starts it locks the folder (the file `lock` in it) for as long as the program runs. A second program that starts the same agent waits up to 5 seconds for the folder (a program that is just ending, as in a restart, lets go of it), then stops with a panic, `the agent RefundBot already runs in another program: its folder agents/RefundBot is locked`, as a second database server refuses a data folder in use. The lock is the operating system's, so a program that crashed holds none.

| Mode | Capability calls | Limits and approvals |
|---|---|---|
| `observe` | Run normally | Evaluated and **reported only** ("would have blocked 3 refunds this week") |
| `suggest` | Every call waits for a person to approve it | Shown alongside each request |
| `enforce` | Run automatically when allowed | **Enforced**: blocked calls fail, approvals pause |

An agent without a `mode` runs in `Enforce` mode: its limits and approvals always act.

The settings are worked out when the program is compiled, with paths relative to the folder of `kavach.toml`. So an executable made by `kavach build` keeps the mode, audit log and state file it was built with.

Separately, `kavach run --dry-run` runs any program **without executing capability calls**. Each call is checked against limits, logged, and answered with a simulated result. Use it to test agents safely.

A simulated result is a value of the method's return type, generated like a test input (11.3) from a fixed seed, so every dry run of a program gives the same results. `ask<T>` is answered the same way, with a value of `T`, so no model is called and nothing costs. Each simulated call is printed to the error output:

```
[dry run] Payments.refund(order_id: 1042, amount: 1499) -> Receipt(id: "a")
[dry run] Payments.refund(order_id: 1043, amount: 90000) refused: limit broken: amount <= 10_000
```

A call a limit or an approval refuses fails as it would in a real run, and says so on the line.
