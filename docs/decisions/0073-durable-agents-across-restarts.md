# 0073. Durable agents: events set aside, one program per agent, an audit log that must be written

- **Status:** Accepted (2026-11-05, Mithlesh chose each recommendation)
- **Date:** 2026-11-05
- **Changes:** spec 8.5, 8.6; `runtime/core/src/journal.rs`, `runtime/core/src/agents`, both runtimes

## Context

[architecture/crashes-and-restarts.md](../architecture/crashes-and-restarts.md) left three cases for Mithlesh:

1. **A handler that always panics.** Its event stays unfinished in the journal, so every start runs it again and panics again: the agent can never start while the bug is there.
2. **Two programs run the same agent.** They share its state file and journal and overwrite each other's limit counts, so a `per day` limit no longer holds.
3. **An audit log that can't be written** (a full disk). Each entry is one write; a failed write was dropped silently, so calls went on unrecorded.

## Decision

1. **An event that started 3 times without finishing is set aside.** Each start of an unfinished event is a line in the journal. When the agent starts and an event already has 3 starts, its lines move to the file `set-aside` in the agent's folder, the audit log gets a line saying so, and the agent starts with the rest. Three, so one crash of the machine or one unlucky restart is not enough. Message queues call this a dead-letter queue: AWS SQS has a "maximum receives" before one, and Temporal and Azure Service Bus a maximum delivery count.
2. **The program that starts an agent locks its folder.** It holds an operating system lock on `lock` in the agent's folder (`File::try_lock`, so a crashed program holds none). A second program starting the same agent waits up to 5 seconds (an old program still ending during a restart lets go of it in that time), then stops with a panic naming the folder, as PostgreSQL and SQLite refuse a data folder another process holds. A panic, not an error: two programs on one agent is a mistake in how it is run, as a journal that can't be written is (spec 8.5).
3. **An audit log that can't be written refuses the agent's calls.** The entry that failed (its call has run) is printed to standard error. Each later call first writes a "writable again" line to the log; until that works, the call is refused with `Denied` and doesn't run. The rest of the program goes on. This is `auditd`'s choice to stop what would go unrecorded, kept to the agent rather than the whole program.

## Consequences

- A poison event no longer keeps an agent from starting, and it isn't lost: it waits in `set-aside` for a person.
- The lock needs Rust 1.89 or later; the repository pins its Rust in `rust-toolchain.toml` (1.99.1).
- Every agent call can now fail with `Denied` because of the audit log, as it already can because of a limit.
- Sending a set-aside event again, and listing what is set aside (`kavach agents set-aside`), are left for later.

## Alternatives considered

- **Retry forever:** nothing is ever dropped, but the agent can't start while the bug is there.
- **Give up after one panic:** a crash of the machine in the middle of an event would drop it, which durable handlers exist to prevent.
- **Allow two programs per agent:** limits would no longer hold.
- **On a full audit log, stop the program** (`auditd`'s `halt`) or **go on and warn once** (`ignore`): the first stops unrelated work too; the second lets calls happen unrecorded.
