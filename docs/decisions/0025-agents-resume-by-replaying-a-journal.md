# 0025. Agents resume after a crash by replaying a journal

- **Status:** Accepted
- **Date:** 2026-11-01

## Context

An agent's handler can stop half way: the program crashes, the machine restarts, a deploy replaces it. If a refund went out and the email didn't, running the handler again from the start would refund twice; not running it again loses the email. Spec 8.3 promises durable handlers.

Kavach code can only reach the world through capabilities (chapter 8), and `ask<T>` goes through `Ai`. Everything else a handler does is computation on its arguments and on what those calls answered. So a handler's run is fully decided by its arguments and the answers of its capability calls.

## Decision

**A durable agent writes a journal, and replays unfinished events when it starts.**

1. An agent is durable when `kavach.toml` gives it a journal: `[agents.RefundBot] journal = "state/refund-bot.journal"`.
2. The journal is a file of JSON lines, appended to and flushed as the handler goes:
   - `event`: when a handler starts, its number, its name and its arguments;
   - `call`: after each capability call or `ask`, the call and its outcome (the value, or the error);
   - `done`: when the handler returns.
3. `Bot.start(…)` first runs again every event that has no `done`, in order. Its handler starts from the beginning with the recorded arguments. A capability call that has a recorded outcome gives that outcome back without running, so no effect happens twice; the first call without one runs for real, and so does everything after it.
4. If a replayed handler makes a different call than the journal recorded (the program changed between the runs), the event stops with a panic that names the event and both calls. The journal is never guessed at.
5. A finished event is kept in the journal until the journal is compacted (later), so the file shows what happened.

## Consequences

- A crash in the middle of a handler loses nothing and repeats nothing, without the program doing anything.
- Replaying needs no snapshot of memory: only arguments and answers, which are values Kavach can write as JSON.
- Calls made by a replayed handler before its first unanswered call do not reach the audit log again.
- Approvals that wait across restarts need more: the event has to stop, and its caller has to see that it waits. That is left as an open question in spec 8.

## Alternatives considered

- **Snapshots of the running handler** (continuations): exact, but large, tied to one build of the program, and hard in native code.
- **Making every capability call idempotent** (a key per call, passed to the outside system): needs every outside system to support it, and still repeats the calls.
- **No durability, only the audit log:** a person would have to read the log and finish each broken event by hand.
