# 11. Testing

Conformance tests for spec chapter [11. Testing](../../../docs/spec/11-testing.md): the generated tests `kavach test` runs. Format: [../README.md](../README.md).

| Spec | Test | Rule | Expects |
|---|---|---|---|
| 11.2 | [panics-are-found.kav](panics-are-found.kav) | Every function is called with generated arguments; a panic is a failure. | tests fail |
| 11.5 | [capability-errors-are-tried.kav](capability-errors-are-tried.kav) | The program's own capabilities are faked: a call gives a generated value or a declared error, so error paths are tried. | tests fail |
| 11.5 | [capability-fakes-run-the-success-path.kav](capability-fakes-run-the-success-path.kav) | A function that handles every error of a faked capability passes. | tests pass |
| 11.5, 10.13 | [env-fake-stays-the-same-in-a-case.kav](env-fake-stays-the-same-in-a-case.kav) | In a generated test, `Env` is faked: each variable is not set or set to generated text, the same for the rest of the case. | tests pass |
| 11.5 | [fs-fake-is-in-memory.kav](fs-fake-is-in-memory.kav) | In a generated test, `Fs` is a file system in memory: what a case writes, it reads back, and nothing reaches the disk. | tests pass |
| 11.5 | [fs-fake-disk-can-be-full.kav](fs-fake-disk-can-be-full.kav) | In a generated test, the fake disk is full for some paths, so code that can't handle a failed save is found. | tests fail |
| 11.5 | [net-fake-never-reaches-the-network.kav](net-fake-never-reaches-the-network.kav) | In a generated test, `Net` never reaches the network: a connection fails with `Refused` or `TimedOut`, or reaches a fake server that answers HTTP with HTTP. | tests pass |
| 11.5 | [clock-fake-starts-at-a-fixed-time.kav](clock-fake-starts-at-a-fixed-time.kav) | In a generated test, the clock starts at 2026-01-01 00:00 UTC, and `sleep` moves it forward at once. | tests pass |
| 11.5 | [clock-fake-moves-with-every.kav](clock-fake-moves-with-every.kav) | In a generated test, each round of `time.every` moves the clock forward at once, as `sleep` does. | tests pass |
| 11.2 | [require-filters-inputs.kav](require-filters-inputs.kav) | Generated arguments that break a `require` are not used. | tests pass |
| 11.2 | [errors-are-not-failures.kav](errors-are-not-failures.kav) | Returning a declared error is not a failure. | tests pass |
| 11.2 | [ensure-is-checked.kav](ensure-is-checked.kav) | Every result of a generated call must meet `ensure`. | tests fail |
| 11.3 | [invariants-guide-generation.kav](invariants-guide-generation.kav) | Generated records never break their invariants. | tests pass |
| 11.3 | [sample-makes-values.kav](sample-makes-values.kav) | A type that implements `Sample` gets its generated values from its own `sample`. | tests pass |
| 11.9 | [endless-loops-fail.kav](endless-loops-fail.kav) | A call that doesn't finish within its time budget is a failure. | tests fail |
| 11.2 | [bytes-are-generated.kav](bytes-are-generated.kav) | Functions that take `Bytes` get generated `Bytes` arguments. | tests pass |
| 11.5 | [clipboard-fake-starts-empty.kav](clipboard-fake-starts-empty.kav) | In a generated test, the clipboard is a fake in memory: it starts empty, and what is written is read back. | tests pass |
| 11.5 | [clock-fake-moves-with-after.kav](clock-fake-moves-with-after.kav) | In a generated test, `time.after` moves the clock forward at once, as `sleep` does. | tests pass |
