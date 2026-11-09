# 0062. What crashes, restarts and clock changes do

- **Status:** Accepted (Mithlesh's choices, 2026-11-04; revised the same day to do what established languages and systems do, at Mithlesh's request)
- **Date:** 2026-11-04
- **Changes:** spec 5.6, 8.4, 8.5, 14.4; [architecture/crashes-and-restarts.md](../architecture/crashes-and-restarts.md)

## Context

Mithlesh asked (2026-11-04) that durations such as `3.minutes` behave correctly when the program or the machine crashes and restarts, sleeps, or has its clock or time zone changed, and for every such case, with memory and resources, to be written down. [architecture/crashes-and-restarts.md](../architecture/crashes-and-restarts.md) lists them. Five were plain bugs and are fixed. Seven needed a choice.

Mithlesh then asked for each choice to follow what other languages and systems do, since a behaviour of Kavach's own would confuse people and breed bugs. The research is at the end; two choices changed after it.

## Decision

1. **A wait in a durable handler counts what passed while the program was down.** The journal records when the wait ends, as a UTC moment. After a restart the handler waits only until then, or not at all if it has passed. The same holds for `every` and timeouts in handlers.
2. **A durable handler's clock reads are journaled.** `time.now_ms()`, `today()` and `now()` inside a durable handler are recorded like a call's outcome, so a run after a crash sees the same time and makes the same calls (spec 8.5).
3. **Ctrl+C, `SIGTERM` and closing the console end the program as `cli.exit` does.** The end hooks run, and output, logs and traces are flushed. The exit code is 130 for Ctrl+C and 143 for `SIGTERM`. A second Ctrl+C kills at once.
4. **A panic stops only its task**, as spec 5.6 already says, the way Tokio and Java contain a task's or thread's panic. A task that waits for it gets an error from `.wait()` (9.1 gives `T ! Error`), and a panic in the main task still ends the program with code 101. A handler's panic is recorded in the audit log. An agent that serves each request in a task of its own, as Go's HTTP server does, keeps serving. (First written as "the handler's caller sees an error", which a handler whose error type isn't `Error` can't type; reworded with Mithlesh's approval, 2026-11-04.)
5. **Waits and limit windows use the operating system's clock that never jumps, as Rust, Go, Java and Python do.** Clock and time zone changes don't move them. Whether the time the machine slept counts is the system's own rule: it doesn't on Linux and macOS, and it does on Windows. Spec 14.4 says so.
6. **A clock set forward while the program was down loosens limits, and spec 8.4 says so.** Nothing on the machine can tell such a jump from real time passing.
7. **A call cut off by a crash runs at least once.** Spec 8.5 says such a call may run twice. Each capability call in a durable handler gets an idempotency key that stays the same in the run after a crash, so a service can drop the repeat. Limits count a call when it starts, so a crash can't leave it uncounted.

## Consequences

- A handler that waits a day between steps finishes even if the program restarts every day.
- Replays make the same calls, as the journal requires.
- Stopping a service with `SIGTERM` (Docker, systemd) loses no logs or traces.
- One bad request no longer stops an agent that serves each request in its own task.
- The journal gets two more kinds of line: a wait's end and a clock read.
- A capability gains an idempotency key that it can read.
- Kavach has no wall-clock wait (`sleep_until`) and no cron jobs, as the standard libraries of Java, Go, Python, JavaScript and Rust have none: a job at a time of day belongs to the operating system's scheduler (cron, systemd timers, Task Scheduler) or a package.

## Alternatives considered

- **Start a wait again after a restart.** Simple, but a long wait may never finish.
- **Always read the real time in a run after a crash.** Then handlers must cope with taking a different path, and the journal can't check that the calls match.
- **Let the program handle Ctrl+C itself.** Possible later as an event; the clean end comes first.
- **End the program on any panic.** It is simpler, but one bad request stops every agent the program runs.
- **Give a handler's caller a `Panicked` error** whatever the handler's error type. Every handler call would gain an error its signature doesn't show.
- **Count time asleep on every system** (first chosen, then dropped). Linux made `CLOCK_MONOTONIC` count suspend in 4.17 and reverted it soon after, because systemd and other programs broke. Rust and Go considered the same switch and stayed with the system's clock, and Go reverted a Windows change at Microsoft's request. A behaviour of Kavach's own here would surprise more than it helps.
- **Warn in the audit log when the clock seems to have jumped** (first chosen, then dropped). No established system does it, and the warning can't tell a jump from a long stop.
- **Keep saved calls a window longer after a restart.** Never looser, but every restart makes limits stricter for up to a month.

## Research (2026-11-04)

| Choice | What others do |
|---|---|
| 1, 2 | Temporal's durable timers fire at the moment they were set for, after any restart, and its workflows read the time through `workflow.Now()` so that replay takes the same path ([Temporal Go guide](https://docs.temporal.io/dev-guide/go/durable-execution)) |
| 3 | The JVM runs its shutdown hooks on `SIGINT`, `SIGTERM` and `SIGHUP`; Python turns Ctrl+C into `KeyboardInterrupt`, which runs `finally` blocks; Go, Rust and C stop at once unless the program asks for the signal |
| 4 | Go's HTTP server recovers a panic in one request and goes on; a Tokio task's panic reaches only its `JoinHandle` |
| 5 | Linux 4.17's switch of `CLOCK_MONOTONIC` to count suspend was reverted; Rust ([#87907](https://github.com/rust-lang/rust/pull/87907)) and Go ([#24595](https://github.com/golang/go/issues/24595), [#35482](https://golang.org/issue/35482)) stayed with the system's clock; Swift offers both clocks |
| 7 | Temporal activities run at least once and are retried, with the activity's id as the idempotency key |
| No cron | Java, Go, Python, JavaScript and Rust have no calendar scheduler in their standard libraries (Quartz, APScheduler, `robfig/cron` are packages); cron and systemd handle daylight saving's skipped and repeated hours themselves |

