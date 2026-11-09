# 14.15 std.random

Random numbers, behind the `Random` capability (8.1), written in Kavach (`stdlib/random/random.kav`).

```kavach
use std.random

fn roll() -> Int uses Random {
    random.int(1, 7)            // 1 to 6: the end is left out, as in `1..7`
}
```

| Function | Gives |
|---|---|
| `random.int(from, until) -> Int` | A whole number from `from` up to, not including, `until`; each equally likely. `until` must be greater than `from` (a `require`) |
| `random.float() -> Float` | A number from `0.0` up to, not including, `1.0` |
| `random.bool() -> Bool` | `true` or `false`, each half the time |
| `random.pick<T>(items) -> T?` | One of `items`, each equally likely; `none` when there are none |
| `random.shuffle<T>(items) -> [T]` | A copy of `items` in a random order, every order equally likely |
| `random.bytes(n) -> Bytes` | `n` random bytes. `n` is from 0 to 65,536 (a `require`), as in browsers' `crypto.getRandomValues`; for more, call it again |

Every function needs `uses Random`. The numbers come from the operating system's cryptographically secure generator, so they are safe for tokens, keys and passwords; there is no weaker, faster kind to pick by mistake.

Under `kavach test`, the `Random` fake (11.5) gives numbers from the test run's seed instead, so a run can be repeated exactly (11.9).

`std.random` asks the runtime for 64 random bits at a time, with `random_bits()`, which only the standard library may call. Everything else is Kavach code in the module: a whole number in a range takes bits until they fall in an unbiased span, so no value comes up more often than another.
