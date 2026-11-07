# stdlib/random/

`std.random`: random numbers behind the `Random` capability ([spec 14.15](../../docs/spec/14-standard-library/15-random.md)). The runtime gives 64 bits at a time from the operating system's secure generator, or from the test run's seed under `kavach test`; everything else is Kavach code.

| File | Contents |
|---|---|
| `random.kav` | `random.int`, `random.float`, `random.bool`, `random.pick`, `random.shuffle`, `random.bytes` |
