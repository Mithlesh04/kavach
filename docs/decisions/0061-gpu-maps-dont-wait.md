# 0061. A GPU `map` whose list stays on the GPU doesn't wait for it

- **Status:** Accepted (Mithlesh's choice, 2026-11-04)
- **Date:** 2026-11-04
- **Changes:** spec 15.2

## Context

Each GPU `map` waited for the GPU to finish, to read its stop flag, so a panic in a `gpu fn` was reported at the `map` before the program went on. On a game-style loop of 1 million particles, 300 frames took 352 ms, against 330 ms for C++ on one core ([benchmarks/gpu](../../benchmarks/gpu/README.md#frames-records-that-stay-on-the-gpu)), and the wait looked like the cost: about 0.8 ms per `map` whatever its size. Spec 15 listed it as an open question.

## Decision

**A `map` whose list stays in GPU memory doesn't wait.** Its work is queued and the program goes on. A panic in its `gpu fn` is reported when the program next reads the list on the CPU, when another GPU call waits for its own result, or when the program ends: with the same message and the `gpu fn`'s line as before, but after the code that ran in between.

A `map` whose result the CPU reads at once (one that doesn't stay) still waits, as it must for the result.

## Consequences

- Frames run at the GPU's speed: a loop of maps of one list waits only when the CPU reads the list. Measured after the change: 300 frames are 30–110 ms faster, run side by side with the build before. Most of the 0.8 ms was not the wait: the first `map` waits about 235 ms for the GPU to open, once per program, and the other 299 frames take about 130 ms, 0.43 ms each.
- A panic can show after later code ran (a `print` after the `map`, say). The report still names the `gpu fn`'s line, so the cause is as easy to find.
- The runtime keeps the stop flags of the maps that didn't wait, and reads them at those points, oldest first.

## Alternatives considered

- **Keep waiting.** Every panic at its `map`, and each frame waits for the last before it starts.
- **Skip the wait only in loops.** It needs the compiler to find frame loops, and still waits outside them; Mithlesh chose the simpler rule.
