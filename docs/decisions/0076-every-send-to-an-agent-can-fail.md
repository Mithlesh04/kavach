# 0076. Every send to an agent can fail

- **Status:** Accepted (2026-11-07, Mithlesh chose the recommendation)
- **Date:** 2026-11-07
- **Changes:** spec 5.6, 8.6, 8.8; the checker; both runtimes

## Context

Spec 5.6 promised that an agent handler that panics is stopped, logged in the audit log, and the agent keeps serving other requests. Neither runtime did it. A send (`bot.request(order)`) runs the handler in the sender's task, so the handler's panic stopped the sender: the whole program, when that is `main`. And a handler declared without `!` gave a plain value, so the sender had nothing to receive in its place.

Since [decision 0065](0065-a-panic-stops-only-its-task.md), a panic can be stopped where it should be, in both runtimes.

## Options

1. **Every send can fail.** A send gives `T ! Error`, whatever the handler declares. When the handler panics, the panic is written to the audit log and the send fails with `panicked: <message> at <file>:<line>`, as `.wait()` does for a task that panicked (9.1). A handler's own error comes out as before, inside `Error`, which `match` can take apart (decision 0050). Every send needs `try`, `catch` or `match`.
2. **A panic stops the sender.** No language change: the audit log records the panic, then it stops the sender's task, and the program if that is `main`. Spec 5.6 changes to say so.
3. **A send fails only when its handler can.** A handler with `!` turns a panic into an error; one without stops the sender. Two rules.

## Decision

Mithlesh chose option 1 (2026-11-07). Why:

- **It keeps the promise of 5.6.** An agent is a long-running service; one bad event must not take down the program that serves the rest.
- **It is the rule tasks have.** A panicking task's `.wait()` fails with `panicked: …` (decision 0065); a send is a call into the agent's code, and gets the same.
- **A send already waits on things that fail**: an approval that runs out of time, a refused call. Handling its failure everywhere is honest about that.

## Consequences

- `bot.request(order)` is `T ! Error`. Code that used it as a plain value is a compile error until it says `try`, `catch` or `match`.
- A handler's panic is printed, as any panic is, and written to the audit log: `request panicked: <message> at <file>:<line>`. In a durable agent the event stays unfinished in its journal, so it runs again at the next start, and after 3 starts without finishing it is set aside ([decision 0073](0073-durable-agents-across-restarts.md)).
- A panic in the handler's own code is stopped at the send, in the sender's task; values its frames held are not freed, as for a task (decision 0065).
