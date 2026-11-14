# 11. Testing

Most languages make you write a test file for every scenario: empty list, negative number, network down, AI says no. Kavach aims for **close to zero hand-written test files**. You already describe your code with types, contracts and capabilities. `kavach test` turns that description into tests.

## 11.1 What you write, what Kavach tests

| You write | Kavach generates and checks |
|---|---|
| Types and invariants (2, 7.3) | Valid values of every type, including edge cases |
| `require` / `ensure` (7.1, 7.2) | Property tests: many inputs that meet `require`; every result must meet `ensure` |
| `example` clauses (7.6) | Concrete cases that also serve as documentation |
| Capabilities (8) | Fake implementations that return every success and every declared error |
| Agents (8.3) | Scenario exploration: AI answers, approvals, limit windows |
| `test` blocks (11.8) | Only multi-step stories the above can't express |

Nothing in the first five rows is extra work. Types, contracts and capabilities are already part of the code. Writing them makes the code clearer **and** makes it tested.

## 11.2 Every function is tested

For each function, `kavach test`:

1. generates arguments of the parameter types (11.3),
2. keeps only arguments that satisfy every `require`,
3. calls the function, with capabilities faked (11.5),
4. checks the result.

A run **fails** when:

- an `ensure` is false,
- an invariant is broken,
- the function panics (index out of range, overflow, division by zero, failed contract of a function it calls),
- the function does not finish within its time budget (11.9).

Returning a declared error is **not** a failure. Errors are values (chapter 5), so `! BankError` means `InsufficientFunds` is an expected result.

```kavach
pub fn withdraw(account: Account, amount: Decimal) -> Account ! BankError
    require amount > 0
    ensure result.balance == account.balance - amount
{ … }
```

With no test file, `kavach test` checks `withdraw` with zero balances, huge amounts, amounts with many decimal places and amounts equal to the balance. It confirms the result always satisfies the `ensure` and the `Account` invariant.

A function with no contract and no example is still checked: it must not panic for any input. The report lists it as **weakly tested** (11.7).

Some functions get no generated arguments. The report lists each as **not generated**, with the reason:

- a generic function, or a method of a generic type;
- a parameter whose values can't be generated, such as a record holding a `Shared`, unless its type has a `Sample` (11.3);
- a capability with no fake yet (11.5);
- a C function (10.7), or a function a generated case found reaching C or raw memory (`alloc`, `*p`, 6.4): they run only in native code.

When no generated input meets a function's `require`, its `example`s are what checks it. The report says "checked by its examples only", or warns when it has none.

## 11.3 Generated values

Every type gets a generator automatically. No code is needed.

| Type | Values always tried first, then random ones |
|---|---|
| `Int`, `Decimal` | `0`, `1`, `-1`, smallest, largest, values next to every number in a `require` |
| `String` | `""`, one character, whitespace, non-Latin text (`"नमस्ते"`), very long |
| `[T]`, `Map`, `Set` | empty, one item, many items, duplicates |
| `T?` | `none` and a value |
| Unions | every variant |
| Records | generated fields; values that break an `invariant` are never produced |

`require` and `invariant` guide generation. Simple conditions (`amount > 0`, `items.len() <= 10`) are used to generate valid values directly instead of throwing invalid ones away.

When the default generator can't produce good data, a type can make its own values by implementing the `Sample` interface from `std.test` (14.7). Whenever `kavach test` needs a value of the type, it calls `sample` with a `Gen`: random choices, seeded like the rest of the run, so the run can be repeated (11.9).

```kavach
type Email: Sample {
    value: String
    invariant value.contains("@")

    fn sample(gen: Gen) -> Email {
        Email(value: "{gen.word()}@{gen.pick(["example.com", "test.in"])}")
    }
}
```

A value from `sample` that breaks an invariant is not used, and `sample` is called again. A `Gen` is not the `Random` capability (8.1): its choices belong to the test run, not to the real world, so `sample` needs no `uses`.

If Kavach can't find any value that satisfies a `require` after its budget, it reports which condition it could not meet and suggests adding a `Sample` or an `example`.

## 11.4 Shrinking and found cases

When a run fails, Kavach **shrinks** the input to the smallest one that still fails, then prints it:

```
✗ withdraw: ensure failed
    ensure result.balance == account.balance - amount
  smallest failing input:
    account = Account(owner: "", balance: 0.1)
    amount  = 0.2
  saved to tests/found/bank.kavf  (replayed on every run)
```

- Failing inputs are saved in `tests/found/`, one file per module (`tests/found/bank.kavf` for `bank.kav`). These files are **generated**: commit them, but never edit them by hand.
- Each line is one call, with its arguments written the way Kavach code writes values, so the file is easy to read in a review:

  ```
  withdraw(Account(owner: "", balance: 0.1), 0.2)
  Account.deposit(Account(owner: "a", balance: 0), -1)
  ```
- Every run replays all found cases first, so a fixed bug stays fixed.
- A found case whose value breaks a type's `invariant` (7.3), written after the case was saved, can't be built by any program, so it is no case any more: the run takes it out of its file and says so (`dropped a saved case that breaks the invariant …`).
- `kavach test --keep` turns a found case into an `example` clause on the function, so it becomes visible documentation. It does this once the case passes (after the fix): the example says what the call gives now, `== value` or `fails error`, and the case leaves `tests/found/`. A case that still fails stays found, and `--keep` says why.

## 11.5 Capabilities are faked automatically

All side effects go through capabilities (chapter 8). Because of that, `kavach test` can replace every capability with a fake and **never** touches the real world.

| Capability | Automatic fake |
|---|---|
| Your own (`Payments`, `Email`, …) | Each call returns a generated success value **or** one of its declared errors, including `Denied` |
| `Fs` | In-memory file system |
| `Net` | Generated responses, timeouts and connection errors; no real network |
| `Clock` | A controlled clock that can jump forward |
| `Env` | Each variable generated, or not set |
| `Random` | Seeded, so every run can be repeated |
| `Console` | Captured output |
| `Ai` | `ask<T>` returns generated values of `T` (11.6); no model is called |

The built-in fakes in detail:

- `Fs`: each case starts with an empty file system in memory. A file the case didn't write is decided the first time the case reaches it: missing (`NotFound`), or there with generated text; it stays that way for the rest of the case. Writes, appends, folders and removals happen in memory, and folders are made as needed. Like a real disk, the fake one can be full: for some paths, decided the first time the case writes there, every write fails with `Failed(path, "no space left on device")`, so code is tried on what it does when saving fails. `uses Fs("./data")` still limits the paths (8.1).
- `Net`: nothing goes on the network. A `connect` or `connect_tls` either fails with a generated `NetError` (`Refused` or `TimedOut`) or reaches a fake server. The fake server takes what the case writes; a read gives a generated answer, then the end of the stream. A request that starts like HTTP (`GET /path HTTP/1.1`) gets an HTTP response with a generated status (200, 404 or 500) and body, so code built on `std.http` is tried on success and on failure. `listen`, `bind` and `connect_quic` fail.
- `Env`: `env(name)` reads no real variable. Each name is decided the first time the case reads it: not set (the call fails), or set to generated text: empty, a word, a number, a URL or `true`. It stays that way for the rest of the case, and is a secret as a real one is (10.13).
- `Clock`: every case starts at 2026-01-01 00:00 UTC, and `sleep` moves the clock forward at once instead of waiting. A `test` block chooses its own moment with `time.freeze` ([14.4](14-standard-library/04-time.md#the-clock-in-tests)).
- `Random`: each `test` block and `example` draws from a seed made from the run's seed and its name, and each generated case from its arguments, so adding a test doesn't change another's numbers ([14.15](14-standard-library/15-random.md)).

What a case finds (files, errors) is decided from its arguments, so the same case finds the same world again when it is shrunk or replayed (11.4).

```kavach
fn save(note: String) -> String ! FsError uses Fs
    ensure result == note
{
    try fs.write_text("notes/today.txt", note)    // in memory, in a test
    try fs.read_text("notes/today.txt")
}
```

So **error paths get tested without you writing them.** If `refund` declares `! PaymentError`, every caller is run once where it returns `Declined` and once where it returns `Unavailable`.

To use a hand-written fake instead, provide it in a `test` block (11.8).

## 11.6 Agents are explored

For each agent, `kavach test` runs every handler many times while varying everything outside the agent's control:

- **Events:** handler parameters are generated (11.3).
- **AI answers:** `ask<T>` returns generated `T` values. `T` is a checked type (8.4), so this covers **every answer the model could give**, including unhelpful ones (an empty `reason`, `approve: true` on a suspicious note).
- **Approvals:** each approval is tried as approved, rejected and timed out.
- **Limits:** windowed limits are pushed to their edges, with the clock moved across window boundaries.

It checks that:

- no limit is ever exceeded,
- secret values never appear in the audit log, prompts or output,
- handlers never panic and keep their contracts,
- every capability call appears in the audit log.

The result is a short report, not a wall of test names:

```
RefundBot.request   2,000 scenarios   ✓
  ✓ Payments.refund never exceeded amount <= 10_000
  ✓ approval asked for every refund > 5_000
  ✓ no secret reached the audit log
  ! Email.send fails after a refund succeeded in 61 scenarios:
    the customer was refunded but not told. Is this intended?
```

The last line is a **finding**, not a failure: everything declared held, but Kavach shows a behaviour a reviewer should look at.

## 11.7 The test report

`kavach test` ends with what it could and could not check:

```
142 functions   138 ✓   1 ✗   3 weakly tested
weakly tested (no contract or example; only checked for panics):
  orders.kav:40   fn label(status: Status) -> String
never reached:
  billing.kav:88  BankError.Frozen is declared but never returned
```

The goal is not a coverage percentage. The goal is that every **weakly tested** line is a conscious choice.

`kavach test --suggest` asks an AI model to propose `ensure` and `example` lines for weakly tested functions. Suggestions are shown for review and are **never written without the developer accepting them**.

## 11.8 Hand-written tests

Some behaviour is a story across many calls: sign up, place an order, refund it. For these, write a `test` block:

```kavach
use orders.{Order, total_paid}

test "total_paid counts only paid orders" {
    const list = [
        Order(id: 1, amount: 100, status: Status.Paid),
        Order(id: 2, amount: 50),
    ]
    assert total_paid(list) == 100
}
```

- `test` blocks live beside the code they test: in `orders.test.kav` next to `orders.kav`, or in the same file. A story across many modules goes in `tests/`. `test` blocks are left out of non-test builds.
- `orders.test.kav` is part of the module `orders`: it sees the module's private names, as code in `orders.kav` does, and needs no `use orders`. It is checked with the module, and counts as a file of its own for the length limit (12.1).
- A `name.test.kav` with no `name.kav` beside it is a compile error that says which module it should be named after.

```
src/
├── orders.kav
└── orders.test.kav      // test "total_paid counts only paid orders" { … }
```
- A `test` block is a function of its own for its tasks: those it spawned and left running are cancelled when it ends (9.2), so a server it started doesn't keep the run going.
- `assert` shows both sides of a failed comparison.
- Tests run with no capabilities unless given some: `test "fetch" uses Net { … }`. Reaching a capability without it, directly or through a function, is a compile error. The console, the clock and random numbers are always there: a test run captures the output, can freeze the clock (14.4) and seeds the numbers.
- A test can provide its own capability implementations: `test "refund flow" with Payments = FakePayments() { … }`.
- A test given `Fs`, `Net`, `Env` or `Clock` reaches the real one. To get the automatic fake of 11.5 instead, write `with Fs = fake`: an empty file system in memory, never full, so nothing the test writes reaches the disk; `with Net = fake`, `with Env = fake` and `with Clock = fake` likewise.

```kavach
test "reports go to an in-memory disk" uses Fs with Fs = fake {
    try fs.write_text("out/report.txt", "total 3")
    assert (try fs.read_text("out/report.txt")) == "total 3"
}
```

## 11.9 Running tests

| Command | Does |
|---|---|
| `kavach test` | Replay found cases, run examples, generate tests, run `test` blocks |
| `kavach test --deep` | Many more generated cases; for CI at night or before a release |
| `kavach test --seed 4211` | Repeat an exact run |
| `kavach test --changed` | Only what is affected by uncommitted changes (12.3) |

A test, an `example` or a generated case that runs past sixty times `time_budget` (a minute by default) is stopped, and `kavach test` names it and fails, so one waiting for something that never comes can't hang the run.

After its results, `kavach test` names what no person has checked: each function with no `ensure` or `example` ("weakly tested": only checked for panics; a `require` limits its inputs but checks no result), and each module with nothing written to check it, no `example`, `ensure`, `invariant` or `test` block in it or in its `name.test.kav` (11.8). Generated tests still try such code, but only for panics; the list says where a person should look first.

```
weakly tested (no contract or example; only checked for panics):
  src/orders.kav:14   note
nothing checked by hand (no example, ensure, invariant or test) in:
  src/billing/invoice.kav
```

Every run prints its seed. The same seed and the same code always give the same result. So the time budget is counted in work done (loop rounds and calls, scaled from `time_budget`), not in seconds, which would vary between machines.

```toml
[test]
cases = 200              # generated cases per function (default 200)
deep_cases = 10_000      # with --deep
time_budget = "1s"       # per function call
```

## 11.10 Crashes: found, explained, fixed

Kavach has no exceptions (decision [0007](../decisions/0007-errors-as-values.md)). A crash is a **panic** (5.6), and a panic is always a bug. When a generated scenario panics, `kavach test` does not stop at "it crashed". It shows the smallest input that causes it and **suggests concrete fixes**, each with when to choose it:

```
✗ average: panicked (division by zero)
    stats.kav:14   total / items.len()
  smallest failing input: items = []

  How to fix (pick one):
    1. Return an error. Change the signature to `-> Int ! StatsError` and add:
         if items.is_empty() { fail StatsError.EmptyList }
       Choose this if an empty list can come from users, files, the network or AI.
    2. Forbid it. Add: require not items.is_empty()
       Choose this if an empty list here is always a bug in the calling code.
    3. Use a default. Add: if items.is_empty() { return 0 }
       Choose this if 0 is a correct answer for an empty list.

  Apply one: kavach fix stats.kav:14 --option 1   (or the editor's quick fix)
```

Fixes are **never applied automatically**. The developer picks one, because only they know what the input means.

### The rule behind the suggestions

| If the bad value can come from… | Then… | Because |
|---|---|---|
| Outside: users, files, network, AI answers | **return an error** (`! E`, `fail`) | It will happen in production; callers must handle it, and the compiler makes them |
| Only from a bug in our own code | **add a `require`** | The caller has the bug; the contract points to it |
| Nowhere, if a safe default exists | **use the default** | No error to handle at all |

### Suggestions for each kind of crash

| Crash | Suggested fixes |
|---|---|
| Index out of range: `list[i]` | `list.get(i)` with `??` or `if let`; or `require i < list.len()` |
| `.unwrap()` on `none` | `if let`, `match`, or `?? default`; or return an error |
| Integer overflow | A `require` that bounds the inputs; a wider type; `checked_add` (and similar), which returns `none` on overflow |
| Division by zero | `require divisor != 0`; or return an error; or a default |
| A called function's `require` failed | Copy that `require` onto this function, or check the value before the call |
| Time budget exceeded | Show the loop or recursion that did not end, and the input that caused it |
| Explicit `panic("…")` | Turn it into an error variant if the situation can happen in production |

### Finding crash points before running

`kavach audit --panics` lists every place in the code that **can** panic: indexing, `.unwrap()`, division, explicit `panic`, and calls to functions with a `require`. For each one it shows whether generated tests or a contract shows it is safe:

```
stats.kav:14    total / items.len()     ✗ crashes for items = []
orders.kav:31   orders[0]               ✓ safe: require not orders.is_empty()
report.kav:9    config.unwrap()         ? never reached by generated tests
```

A team can require zero `✗` lines before a release.

### In production

If a crash still reaches production, it stays contained: a panic stops only the task it happened in (5.6). An agent handler that panics is logged in the audit log, and the agent keeps serving other requests. The panic message includes the inputs, so the case can be added as a found case (11.4) and the bug fixed for good.

## 11.11 When each part arrives

| Phase | Testing features |
|---|---|
| 1 | `example`, generated values, contract checking, shrinking, found cases, crash fix suggestions, `test` blocks |
| 2 | Automatic capability fakes, agent exploration, `ask<T>` generation |
| 3 | Test report with weak spots, `--suggest`, `--changed`, `audit --panics` |

## Open questions

- Should `kavach test` also mutate function bodies to find contracts that are too weak to catch bugs (mutation testing)?
- How a `test` block scripts specific AI answers and approval decisions, e.g. `with ask<RefundDecision> = [...]`.
- Default `cases` per function: is 200 the right balance between speed and confidence?
