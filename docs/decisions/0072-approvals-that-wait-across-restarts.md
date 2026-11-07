# 0072. Approvals that wait across restarts

- **Status:** Accepted (2026-11-05, Mithlesh agreed to the recommendation as written)
- **Date:** 2026-11-05
- **Changes:** spec 8.3, 8.5, 14.16; the CLI; `runtime/core/src/journal.rs`, both runtimes

## Context

`approve Payments.refund when amount > 5_000 by "finance"` pauses a handler until a person decides (spec 8.3). Today a person answers at the terminal, and with no terminal the call is refused. A real approval can take hours or days. Meanwhile the program may crash, be updated or the machine may restart. Spec 8.5 leaves this open, with two questions (asked 2026-11-02):

1. How does the code that sent the event learn that it waits?
2. How does a person answer later, when no terminal is open?

Durable handlers already replay a journal after a crash (decision 0025), and a `sleep` waits only for what is left after a restart (decision 0062). An approval is the same kind of wait, ended by a person instead of the clock.

## What others do

| System | The wait | The answer | A deadline |
|---|---|---|---|
| Temporal | `workflow.wait_condition(…)` until a Signal sets a field ([approval pattern](https://docs.temporal.io/design-patterns/approval)) | a Signal sent to the workflow's ID, from any client | a timeout on the wait; it "survives any execution disruptions" |
| AWS Step Functions | a task with `.waitForTaskToken` ([callback](https://kindatechnical.com/aws-lambda/wait-for-callback-pattern-pausing-workflows-with-task-tokens.html)) | `SendTaskSuccess` or `SendTaskFailure` with the token | the execution's limit, one year ([wait state](https://docs.aws.amazon.com/step-functions/latest/dg/state-wait.html)) |
| Azure Durable Functions | `WaitForExternalEvent` raced against a durable timer ([pattern](https://markheath.net/post/durable-functions-wait-external-event-timeout)) | `RaiseEventAsync(instanceId, name, data)`; an answer that comes early is kept | the timer; timing out is "a business outcome" ([Learn](https://learn.microsoft.com/en-us/training/modules/orchestrate-durable-functions/4-wait-human-approval-external-events)) |
| Restate | an awakeable: an ID and a durable promise ([docs](https://docs.restate.dev/ai/patterns/human-in-the-loop.md)) | an HTTP call to resolve or reject that ID | a timeout on the promise |
| Inngest | `step.waitForEvent(…, { match: "data.approvalId" })` ([docs](https://www.inngest.com/docs/reference/functions/step-wait-for-event)) | an event whose `approvalId` matches | `timeout: "7d"`; a timed-out wait gives `null` |
| LangGraph | `interrupt(value)`, with a checkpointer ([docs](https://docs.langchain.com/oss/python/langgraph/interrupts.md)) | `Command(resume=…)` on the same `thread_id` | none: it waits forever |

They agree on four things:

- **The wait is saved, with an ID.** The process may stop while it waits; nothing is held in memory.
- **The answer comes from outside, by that ID:** a command, an API call or an event. Whoever delivers the request (email, Slack, a web page) is up to the program.
- **The deadline is a saved timer too.** Running out of time is an ordinary outcome, most often a "no".
- **No caller stays blocked across a restart.** Work is started and named; its result is read later, or written where it can be found.

## Decision

1. **An approval in a durable handler is a journaled wait**, as `sleep` is (8.5). The journal records the request: its ID, the call and its arguments, the role, and the deadline as a moment in UTC. After a restart, the replay reaches the request again and waits for what is left. A request answered while the program was down gets its answer at once.
2. **A deadline on every request**, written `within`, or 7 days if none is written. Running out of time refuses the call with `Denied`, as a rejection does:

   ```kavach
   approve Payments.refund when amount > 5_000 by "finance" within 2.days
   ```

3. **The sender just waits, as for any slow call.** While the program runs, `try bot.request(order)` waits for the handler, approval included; other tasks run meanwhile (9.7). No new error case is needed. After a restart, the original caller is gone. So `start` replays each unfinished event only until it waits for a person, then returns. The event goes on inside the agent, and its result or error is written to the audit log (8.6). A program that sends the same event again after the restart (the same handler, with equal arguments) while it waits joins it: it waits for that event's result, and no second request is made.
4. **A person answers in one of three ways.** Each answer goes to the audit log with who gave it:
   - **At the terminal**, as today, when the program runs in one.
   - **On the command line:** `kavach approvals` lists the requests waiting (ID, agent, call, arguments with secrets hidden, role, when asked, deadline). `kavach approve <id> --as <name>` and `kavach deny <id> --as <name> [--reason …]` answer one. The answer is written into the agent's state folder. A running program sees it within a second; a stopped one finds it when it starts.
   - **In code**, for answers from Slack, email or a web page: `agents.waiting()` lists the requests, and `agents.answer(id, approved: Bool, by: String)` answers one (14.16). These need the capability `Approvals`, which an agent's `can` may never list, so an agent can't approve its own requests.
5. **Request IDs are stable:** the agent, the event's number and the call's place in it (`RefundBot-17-2`), as `agents.call_key()` is. The same request keeps its ID across replays.
6. **Without a journal** (`state: false`, or a program outside a project), approvals wait only while the program runs, as today. A crash ends the wait, and the call is never made.

## Why

- **It reuses what exists.** Journaled waits, UTC deadlines, replay and stable call keys are all built (decisions 0025, 0062). An approval adds a request record and a way to deliver its answer.
- **Code reads the same as before.** A handler with `approve` is written as if a person answered at once, and so is the code that sends it events. Only `within` is new.
- **Nothing is approved by accident.** A timeout, a crash without a journal or a missing terminal all refuse the call. Approving is never the default.
- **Delivery stays the program's choice**, as in Temporal and Restate. Kavach gives the IDs, the list and the answer call; Slack and email are a few lines on top.
- **An agent can't approve itself.** The `Approvals` capability can't be in an agent's `can`, and the checker enforces it.

## Consequences

- `start` no longer runs every unfinished event to its end before returning: an event waiting for a person continues inside the agent.
- Who may answer as `finance` is not checked by Kavach. On the command line it is whoever can write the state folder (the operating system's file permissions). In code it is the program's own sign-in. The audit log records who answered.
- Joining by equal arguments can merge two events that really are separate but alike, and only after a restart. An explicit event key (Temporal's workflow ID) is the alternative if that ever matters.
- Work: the journal's request and answer records, the state folder's answer files, `kavach approvals`/`approve`/`deny`, `agents.waiting`/`answer`, the `Approvals` capability and its checker rule, `within` in the parser and checker, in the interpreter and native code, and conformance tests for each rule.

## Alternatives considered

- **A new error, `Waiting`, on every handler call**, so the sender learns it waits and comes back later. Every capability error type would need the case, and every caller code to handle it. Rejected: it makes ordinary code pay for a rare case.
- **Events sent without waiting for a result** (`bot.send.request(order)` giving a handle, as Temporal's client does). That's more to learn, and most events are answered in seconds. It can be added later beside today's call if programs need it.
- **Approvals only by command line, or only in code.** The command line needs no code, but a team that approves in Slack needs the code path. Both are small.
- **No deadline by default**, as LangGraph does. Then a forgotten request waits forever, and so does its caller. 7 days leaves room for a weekend and a holiday.
