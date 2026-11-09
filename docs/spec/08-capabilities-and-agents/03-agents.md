# 8.3 Agents

An agent is a long-running unit that reacts to events, may use AI, and acts through capabilities under guardrails.

An agent is written in this order: `model`, `settings` (8.5), then `can`, `limit` and `approve`, then its handlers (`on`).

```kavach
agent RefundBot {
    model Model(name: "claude-haiku-5-5")
    can Ai, Payments.refund, Email.send

    limit Payments.refund {
        amount <= 10_000                 // checked on every call
        sum(amount) <= 50_000 per day    // across all calls in the window
        count <= 100 per day
    }

    limit Ai {
        count <= 1_000 per day
    }

    approve Payments.refund when amount > 5_000 by "finance"

    on request(order: Order) ! Error {
        const decision = try ask<RefundDecision>("Should we refund order {order.id}? Note: {order.note}")
        if decision.approve {
            const receipt = try Payments.refund(order.id, order.amount)
            try Email.send(order.customer_email, "Refund {receipt.id} is on its way.")
        }
    }
}
```

### `model`: the AI model its `ask`s use

`model` names the model, and its settings, for every `ask` in the agent's handlers ([decision 0056](../../decisions/0056-the-model-is-chosen-in-code.md)). It is a `Model` value (8.4). To share one between agents, give it a function with no parameters: agents that call the same function are a group, and changing that function changes all of them.

```kavach
fn careful() -> Model {
    Model(name: "claude-opus-5-5", max_tokens: 2000)
}

fn quick() -> Model {
    Model(name: "claude-haiku-5-5")
}

agent Reviewer {
    model careful()          // Reviewer and Auditor use one model
    can Ai
    on check(text: String) -> Verdict ! Error { try ask<Verdict>(text) }
}

agent Auditor {
    model careful()
    can Ai
    on audit(text: String) -> Verdict ! Error { try ask<Verdict>(text) }
}
```

An agent without `model` uses the default of 8.4.

### `can`: what the agent may touch

Handlers have their capabilities inferred. The compiler rejects any handler that needs something not listed in `can`. **This is a compile error, not a runtime surprise.**

### `limit`: hard boundaries on capability calls

Inside a `limit` block, the arguments of the limited method are in scope by name.

| Rule | Meaning |
|---|---|
| `amount <= 10_000` | Every single call must satisfy this |
| `sum(amount) <= 50_000 per day` | Total across calls in the window |
| `count <= 100 per day` | Number of calls in the window |
| `tokens <= 200_000 per day` | In `limit Ai`: the tokens (in and out) the window's earlier `ask`s took, with those still waiting for their answer counted as the agent's settings say (below). Once they pass the limit, the next `ask` is refused; the one that crosses it still runs, since an answer's size is known only after it. Anywhere else, `tokens` is a compile error. |

Windows: `per minute`, `per hour`, `per day`, `per week`, `per month`.

Windows are rolling, not calendar periods. Before each call, a limit counts the calls of the last 60 seconds (`per minute`), 1 hour, 24 hours (`per day`), 7 days (`per week`) or 30 days (`per month`). Time is measured on a clock that never jumps, so time zones, daylight saving time and the lengths of months don't change a window: `per day` never resets at midnight. With a state file (8.5), each call is saved with its time in UTC, so the windows carry on across restarts, a reboot of the machine too. A saved call the clock says is still to come (the clock was set back while the program was down) counts as made just now, so a limit never loosens. A clock set forward while the program was down does the opposite: saved calls look older and can leave their window early, and nothing on the machine can tell such a jump from time passing ([decision 0062](../../decisions/0062-crashes-restarts-and-clocks.md)).

A call that would break a limit is **not executed**. It fails with the error `Denied(reason)`, which every capability call can return. Limit counters live in the agent's state store (see 8.5), so they survive restarts.

**`ask`s under way.** Many `ask`s can run at once (tasks, 9.1), and each answer's size is known only when it comes. So `tokens` also counts the `ask`s still waiting for their answer. The agent's settings choose how, with `asks_under_way` (8.5):

| `asks_under_way` | An `ask` under way counts as | So |
|---|---|---|
| `UnderWay.Average` (the default) | The average size of the window's answered `ask`s | A new `ask` the limit would refuse waits while others are under way, then is checked again with their real sizes. When others are under way and none in the window has answered yet, their size isn't known, so it waits for the first answer. The total passes the limit by about one answer at most, as when `ask`s run one at a time. Near the limit, `ask`s run one after another. |
| `UnderWay.Largest` | The most it can take: for each of its 3 tries (8.4), its prompt's length in bytes (no token is shorter than a byte) and its model's `max_tokens`. The new `ask` counts this way too. | The total doesn't pass the limit, except by text the provider adds to a request, such as its own instructions for structured output. But `ask`s are refused well before it, as most answers are much shorter than `max_tokens`. With the default `max_tokens` of 4,096, each `ask` counts as at least 12,288 tokens, so a smaller limit refuses every one: give the agent's model a smaller `max_tokens`. |
| `UnderWay.NotCounted` | Nothing | Nothing waits. 10 `ask`s started together under `tokens <= 100 per day` all run, and can take 700 tokens. For a limit kept as a rough guide. |

```kavach
agent Writer {
    settings AgentSettings(asks_under_way: UnderWay.Largest)
    can Ai
    limit Ai {
        tokens <= 200_000 per day
    }
    on draft(topic: String) -> Draft ! Error { try ask<Draft>("Write about {topic}.") }
}
```

In `observe` mode (8.5) nothing waits: the limit is only reported.

### `approve`: a person decides

```kavach
approve Payments.refund when amount > 5_000 by "finance"
```

When the condition is true, the handler **pauses** and an approval request is made for the `finance` role. The handler resumes when someone approves, or the call fails with `Denied` if they reject it or the request runs out of time ([decision 0072](../../decisions/0072-approvals-that-wait-across-restarts.md)).

- **A deadline.** `within` says how long a request may wait; without it, 7 days. When it passes, the call is refused with `Denied`, `not approved by finance within 2 days`:

  ```kavach
  approve Payments.refund when amount > 5_000 by "finance" within 2.days
  ```

- **The wait survives restarts.** In a durable handler (8.5), the request is journaled: its number, the call and its arguments, the role and the deadline (a moment in UTC). After a crash or a restart, the event runs again up to the request and waits for what is left; an answer given while the program was down is there at once. Without a journal, the request waits only while the program runs.
- **The sender just waits.** `try bot.request(order)` waits for the handler, approval included, as for any slow call; other tasks run meanwhile (9.7). After a restart the sender is gone: `start` runs each unfinished event until it waits for a person, then returns, and the event goes on inside the agent, its result or error written to the audit log (8.6). It goes on like a task spawned where `start` was called: when that function returns, it is cancelled and stays unfinished in the journal (9.2). Sending the same event again (the same handler, with equal arguments) while it waits joins it, so no second request is made.
- **Request numbers.** Each request has a number that stays the same across restarts: the agent, the event's number and the call's place in it, `RefundBot-17-2`, as `agents.call_key()` (14.16).
- **Answering.** A person answers at the terminal while the program runs in one; on the command line, `kavach approvals` lists the requests waiting, and `kavach approve RefundBot-17-2 --as asha` or `kavach deny RefundBot-17-2 --as asha --reason "duplicate"` answers one (10.5); in code, `agents.waiting()` and `agents.answer(id, approved, by)` (14.16) let a program take answers from a web page, chat or email. Answering in code needs the capability `Approvals`, which an agent's `can` may never list, so an agent can't approve its own requests. Each answer is written to the audit log with who gave it. Who may answer as `finance` is the operating system's (who can write the agent's folder) or the program's (its own sign-in) to decide.

### The order of the checks

Every capability call an agent's handler makes goes through the same steps, in this order:

1. **`can`** was already checked at compile time: a call the agent may not make never compiles.
2. **`--dry-run`** (8.5): the call is not executed; it is checked and logged, and answered with a simulated result.
3. **Limits**: each rule of the method's `limit` block, with the call's arguments, and the totals for its window including this call. A broken rule refuses the call (`Denied`). Limits come before approvals, so no one is asked to approve a call that would be refused anyway.
4. **Approval**: if an `approve` condition is true, the handler pauses until a person decides. A rejection, or a request that times out, refuses the call (`Denied`).
5. **The call** runs on the agent's implementation. Only a call that runs counts toward `sum` and `count` totals.

The agent's mode (8.5) decides what steps 3 and 4 do: in `observe` mode they are only reported. Every call, refused or not, is written to the audit log (8.6).

### `on`: event handlers

`on name(params) { … }` defines an event the agent handles. Each event becomes a method on the running agent (8.8).
