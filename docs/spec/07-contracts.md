# 7. Contracts

A contract states what a function expects and promises, in one line each. When AI writes the body, people review the contract, which is short, and Kavach checks that the body keeps it.

## 7.1 require: what the caller must ensure

```kavach
pub fn withdraw(account: Account, amount: Decimal) -> Account ! BankError
    require amount > 0
{
    …
}
```

If a `require` is false when the function is called, the **caller** has a bug.

## 7.2 ensure: what the function promises

`result` refers to the return value.

```kavach
pub fn withdraw(account: Account, amount: Decimal) -> Account ! BankError
    require amount > 0
    ensure result.balance == account.balance - amount
{
    if amount > account.balance {
        fail BankError.InsufficientFunds(needed: amount - account.balance)
    }
    account with { balance: account.balance - amount }
}
```

- `ensure` is checked only when the function succeeds. It does not apply when the function returns an error.
- Parameters are values (chapter 6), so `account.balance` in an `ensure` is always the balance *before* the call. No special "old value" syntax is needed.

## 7.3 invariant: always true for a type

```kavach
type Account {
    owner: String
    balance: Decimal

    invariant balance >= 0
}
```

An invariant is checked whenever a value of the type is created or changed. An `Account` with a negative balance cannot exist.

## 7.4 How contracts are checked

| Build | Checked |
|---|---|
| `kavach run`, `kavach test`, debug builds | Every contract, at run time |
| `kavach build --release` | Contracts of `pub` functions and all invariants (default) |

Configure release checking in `kavach.toml`:

```toml
[build.release]
contracts = "public"    # "all" | "public" | "none"
```

A failed contract is a **panic** (5.6) with a message that shows the contract and the values involved:

```
contract failed: require amount > 0
  in withdraw(account, amount) at bank.kav:12
  amount = -5
```

### Static checking (roadmap)

- **Phase 3:** the compiler proves simple contracts at compile time and removes their run-time checks.
- **Phase 4:** an SMT solver can prove harder contracts for critical code (`kavach verify`).

Contracts written today keep working unchanged as checking gets stronger.

## 7.5 Contracts must be pure

A contract must not change anything or use any capability. It can only read values and call functions that have no `uses` clause.

"For all" and "there exists" need no special syntax. Use collection methods:

```kavach
ensure result.all(.amount > 0)
require orders.any(.status == Status.Paid)
```

## 7.6 example: a concrete case

`require` and `ensure` describe every call. `example` shows one specific call and its expected result:

```kavach
pub fn withdraw(account: Account, amount: Decimal) -> Account ! BankError
    require amount > 0
    ensure result.balance == account.balance - amount
    example withdraw(Account(owner: "Asha", balance: 100), 30).balance == 70
    example withdraw(Account(owner: "Asha", balance: 10), 50) fails InsufficientFunds(needed: 40)
{ … }
```

- `example <call> == <value>` (or any `Bool` expression using the call) must be true.
- `example <call> fails <error>` must return that error.
- Examples come after `ensure` and follow the same purity rule (7.5).
- `kavach test` runs every example (chapter 11). Examples also appear in editor hover text and in docs, so they are documentation that can't go out of date.
- In an example, a call that can fail needs no `try`: if it fails, the example fails. That is why `withdraw(…).balance == 70` above can read the balance directly.
- Examples are left out of non-test builds.

## 7.7 A call that always breaks a contract

When the compiler knows what a call passes (the known values of 1.8), and a `require` of the function compares a parameter with a number, a call for which that `require` is false is a compile error. It would always panic, so it is found before the program runs, with the contract and the value. Parts joined with `and` are each checked.

```kavach
fn half(x: Int) -> Int
    require x > 0
{
    x / 2
}

print(half(-4))     // error: this call always breaks `require x > 0` of `half`: `x` is -4
```

The same goes for a value a function returns. When the compiler knows it (the last expression of the body, of a branch at the end of it, or a `return`), and an `ensure` that compares `result` with a number is false for it, that is a compile error, even in a branch that may not run: whenever it runs, it panics.

```kavach
fn clamp_low(x: Int) -> Int
    ensure result >= 0
{
    if x < 0 {
        return -1       // error: this always breaks `ensure result >= 0` of `clamp_low`: `result` is -1
    }
    x
}
```

A value the compiler can't know (read from input, worked out in a loop) is still checked when the program runs (7.4).

## Open questions

- Loop invariants for the static checker (Phase 3).
