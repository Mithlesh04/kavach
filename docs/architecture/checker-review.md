# Type checker soundness review

A review of the type checker: step 1, item 1 of the [to-do](../vision/todo.md) (foundation 2). It looks for programs the checker **accepts but should refuse** (unsound), and programs it **refuses but the spec allows** (incomplete). It also says where the checker's rules are written down, and how a fuzzer could find more holes.

**How it was made (2026-11-26, at commit `02d554250`).** Source files and the spec were read. Small probe programs were checked and run with the already-built `target/release/kavach.exe` (`check` and `run`, the interpreter). Nothing was built: no `cargo`, no `kavach build`, so native code was not tried. The [compiler review](compiler-review.md) of the same day is not repeated here.

- **Confirmed** means a probe program showed it.
- **Suspected** means it comes from reading the code only.

## Summary

- **13 confirmed holes** where the checker accepts what it should refuse. **6 of them reach the outside world or a secret**; **5 end in a run-time type confusion** in the interpreter (`panic: expected an integer`, `can't use + on () and ()`).
- **3 confirmed places** where the checker refuses what the spec allows.
- **6 suspected** problems and spec gaps.
- The holes have **three root causes**, not thirteen separate bugs:
  1. **Checks are syntactic, not on value flow.** Secrets, capabilities and function-type limits are checked at some places a value can go, not at all of them (findings 3, 4, 7).
  2. **"Not known yet" means "fine".** A type that is a variable or a type parameter at the moment of a check skips the check, and nothing checks again later (findings 5, 6, 12, 13).
  3. **`Ty::Error` is made without an error.** Some paths give `Ty::Error`, which fits anywhere, without reporting anything (findings 8, 13).
- **Frozen values, distinct types (except `sum`), optionals, number parameters and `match` over unions held up well.** About 15 probes against them were all refused correctly.

### Top 5

1. **Interface methods hide capabilities.** A `pub fn` with no `uses` writes files through an interface value or a generic bound (finding 1).
2. **`json.decode` forges values other modules can't make.** It makes a sealed value's key and opens the sealed value, and sets a `pub(read)` field (finding 2).
3. **Secrets leak** through any method call, list, tuple, `if` branch or later assignment: `print(c.card.upper())` passes (finding 3).
4. **Interface bounds are skipped** when the argument is a type parameter or not yet known, and the program panics with a type confusion at run time (findings 5, 6).
5. **A generic type written without its arguments** fits every instance, so a `Pair<String>` becomes a `Pair<Int>` (finding 9).

## Confirmed: accepted but should be refused

Every probe below is in `checker-probes/` next to this file, under the name given.

### 1. Interface methods hide capabilities (critical)

```kavach
interface Describe {
    fn describe(self) -> String
}

type User: Describe {
    name: String
    fn describe(self) -> String uses Console {
        print("side effect from a pure function")
        self.name
    }
}

pub fn make() -> Describe { User(name: "a") }
pub fn pure_len() -> Int { make().describe().len() }   // no `uses`, yet prints
```

- **Spec 8.1, rule 1:** "A function **without** `uses` is pure. It cannot affect the outside world at all." Rule 2: "A function may only call functions whose capabilities it also has."
- **What happens:** `kavach run` prints. The same works with a generic bound, `pub fn show<T: Describe>(x: T) -> String { x.describe() }` (`cap-generic.kav`), with `uses Fs` and `fs.write_text` (`cap-iface.kav`), and inside a `require` (`contract3.kav`), which breaks spec 7.5 too.
- **Why it matters:** this is the main promise of capabilities. Any library that takes an interface value or a bounded `T` can do I/O with no `uses` in its signature, and `kavach audit` shows nothing.
- **Where:**
  - `capabilities.rs:134-187`: `fn_infos` has no arm for `ItemKind::Interface`, so an interface method has no entry, and `needed` (line 214) gives it no capabilities.
  - `methods.rs:106-127`: the call on an interface value or a `Ty::Param` takes the interface's signature; `calls.rs:217` notes that signature's `def`.
  - `conform.rs:59` (`conforms`) compares parameters, `inout`, errors and `secret`, but not `uses`.
- **Spec gap:** 2.8 and 2.15 say nothing about `uses` on interface methods. A decision is needed: either an interface method lists its `uses` and an implementation may use only that (as an operator method must be pure, 2.16), or making an interface value pays for its methods' capabilities (as making a function value does, 8.1). For generics, the checker could also charge the concrete method's `uses` at each instantiation.

### 2. `json.decode` makes values only their module may make (critical)

```kavach
// session.kav
pub type SessionKey {
    n: Int                       // private: other modules can't make one (2.20)
}
const key = SessionKey(n: 1)
pub type Session { pub user: String  pub roles: [String] }
pub fn open(user: String) -> Frozen<Session, SessionKey> { … .freeze(key: key) }

// main.kav
const s = session.open("asha")
const forged = json.decode<session.SessionKey>(`\{"n": 1}`) catch _ => return
let opened = s.unfreeze(key: forged)
opened.roles.push("admin")       // runs: ["reader", "admin"]
```

- **Spec 2.20:** "A library makes the key a value of a type of its own that no other code can make … A type that is `pub` but has a private field can't be made outside its module either (10.1)." **Spec 10.1:** "A record with a private field can be **created only in its own module**", and for `pub(read)`: "making a record of the type is a compile error".
- **What happens:** both run (`pubread/`). The same project also forges an `Account` with `pub(read) balance` of 1,000,000.
- **Why it matters:** sealed values are the library's tool for "only I change this". `json.decode` is a back door to every type's private fields. Invariants are still checked on decode, so only types whose invariants don't cover the field are open.
- **Where:** the type argument of `json.decode<T>` is checked for being encodable (`json.rs`), not for being constructible in the calling module. `cli.parse<T>` and `ask<T>` likely have the same gap (suspected, S2).
- **Fix direction:** decoding into a type with a private or `pub(read)` field is a compile error outside its module (or its package's `extend`), with a message naming the field.

### 3. Secrets leak through ordinary expressions (critical)

```kavach
const c = Customer(name: "a", card: "4111")
print(c.card.upper())                         // all accepted, all print 4111
print([c.card])
print((c.card, 1))
const pick = if c.name == "a" { c.card } else { "" }
print(pick)
print(Note(text: c.card.trim()))
let s = ""
s = c.card                                    // a later assignment
print(s)
let t = ""
t += c.card
print(t)
```

- **Spec 8.7:** "Fields marked `secret` are tracked by the compiler wherever their value goes. A secret value **must not** flow into `print` …", and "so is a variable set from a secret".
- **What happens:** all accepted (`secret-leaks.kav`, `secret3.kav`), and `run` prints the card number.
- **Why it matters:** the rule is only as strong as its weakest path, and `.upper()`, `.trim()` or `[x]` are the first thing anyone writes.
- **Where:** `secrets.rs:47-82`. `find_secrets` looks only at a field, a name, a secret call, `Binary`, `try` and `catch`. It doesn't look into method calls, list, tuple, dict and record literals, `if` and `match` branches, unary operators or lambdas. A variable becomes secret only where it is declared (`secret_defs`), not when assigned later (`items.rs:333` assigns without marking).
- **Fix direction:** make secrecy a property of the value's flow, not of the syntax. The simplest sound rule: an expression is secret if **any** sub-expression is, except through `reveal(…)` and the few methods whose result can't carry the text (`len()` is a choice to make in the spec). Assignment and `op=` mark the target.

### 4. Capabilities used by code outside a function are dropped

Two places, one cause:

```kavach
type H {
    n: Int = noisy()           // noisy() uses Console
}
pub fn pure_make() -> Int { H().n }          // prints, no `uses`

fn positive(n: Int) -> Bool {                // private: `uses Console` inferred
    print("printed by an invariant")
    n > 0
}
type Count { n: Int  invariant positive(n) } // accepted, prints on each Count
```

- **Spec 8.1** rules 1 and 2, as above. **Spec 7.5:** "A contract must not change anything or use any capability."
- **What happens:** both run and print (`cap-default.kav`, `invariant-cap.kav`). A default *parameter* value is caught; a default *field* value is not.
- **Where:** `items.rs:120-122` checks invariants and `items.rs:132-138` checks field defaults while `current_fn` is `None`. `capabilities.rs:75-95` (`note`) then records the need nowhere: no function, no test. A direct `print` in an invariant is still caught (`calls.rs:226`), but a call to a function whose `uses` is inferred is only caught from the recorded needs (`capabilities.rs:267-273`).
- **Fix direction:** charge a field default to each place the record is made without that field (the construction), as a default parameter is charged to the call. Record an invariant's needs under the type, and report them with the contract message.

### 5. An interface bound is skipped when the argument is a type parameter

```kavach
fn show<T: Describe>(x: T) -> String { x.describe() }
fn outer<U>(x: U) -> String { show(x) }      // U has no bound
print(outer(5))                              // panic: expected an integer
```

- **Spec 2.9:** "`<T: Comparable>` means any type `T` that implements the interface"; "Any other type that doesn't say so is a compile error."
- **What happens:** accepted; the interpreter panics with an internal type error (`param-bound.kav`). Native code was not tried; a missing method at monomorphization is likely a lowering failure or worse.
- **Where:** `calls.rs:187`: `Ty::Var(_) | Ty::Param(_) | Ty::Error | Ty::Never => continue`. For a `Ty::Param`, the comment says "bounded where it was declared", but nothing checks that its declared bounds include the needed one. `Number` and operator bounds through a parameter **are** checked (`numbers.rs`, `op_bounds.rs`; `param-bound2.kav` is refused), so only interface bounds have the hole.

### 6. An interface bound is skipped when the type is not known yet

```kavach
const f = x => show(x)
print(f(5))                                  // panic: expected an integer
```

- **What happens:** accepted, panics (`var-bound.kav`).
- **Where:** the same `continue` at `calls.rs:187`, for `Ty::Var`. The bound is never checked again once the variable is bound. The fix is to queue `(var, bound, span)` and check the queue after the body, or when the variable is bound.

### 7. Two function-type `uses` limits are not compared inside other types

```kavach
fn quiet_only(hs: [fn() -> Int uses Random]) -> Int { hs.sum(h => h()) }
const w: fn() -> Int = loud              // loud uses Console
const ws = [w]
print(quiet_only(ws))                    // accepted; loud runs
```

- **Spec 3.4 / 8.1:** "A function type can limit what its functions may use with `uses`."
- **What happens:** accepted and run (`fnlimit2.kav`). The direct case, `hs.push(w)`, is refused; a dict assigned to a dict type with a limit is accepted too (`fnlimit.kav`).
- **Where:** `infer.rs:194-196` unifies `Ty::Fn(p1, r1, _)` with `Ty::Fn(p2, r2, _)` and ignores the third part, the limit. The limit is only compared by `fn_fits` (`fn_values.rs:31`), which `coerce` calls at the top level only. There are 48 direct `self.inf.unify(` calls in 23 files that skip `coerce`.

### 8. Names that are not values type as `Ty::Error`, with no message

```kavach
use std.json
fn f<T>(x: T) -> Int { T }               // a type parameter as a value
const n: Int = json                      // a module as a value
print(n + f(1))                          // panic: can't use `+` on () and ()
```

- **Spec 3 and 2.10:** a function's result has the written type; `Ty::Error` "fits anywhere, so one mistake is reported once" (`ty.rs:133-135`), which only holds if the first mistake *was* reported.
- **What happens:** no error; run-time type confusion (`name-error.kav`).
- **Where:** `names.rs:133`: `DefKind::Builtin | DefKind::Import | DefKind::GenericParam => Ty::Error` with no `self.error`. A built-in function used as a value is probably the same.
- **A useful invariant to test:** "a file with no errors has no `Ty::Error` in `expr_types`". It would have caught this and finding 13.

### 9. A generic type written without arguments fits every instance

```kavach
type Pair<A> { v: A }
fn launder(p: Pair) -> Pair<Int> { p }
print(launder(Pair(v: "text")).v + 1)    // panic: can't use `+` on text and 1
```

- **Spec 2.9:** "When a field doesn't show them, write them …"; `const d = Box(raw: text)` is a compile error, "nothing says what `I` is". Spec 2.10: signatures are always written out.
- **What happens:** accepted (`noargs2.kav`).
- **Where:** `infer.rs:211-215`, `unify_args`: "Type arguments left out … match anything". `Pair` in a signature should either be an error or mean fresh variables per use, never a wildcard that both sides can pass through.

### 10. `sum(f)` takes any result type

```kavach
type NetId = distinct Int
const s: NetId = [NetId(1), NetId(2)].sum(p => p)      // 3: arithmetic on a distinct type
print(["a", "b"].sum(w => w))                         // ab
print([true, false].sum(b => b))                      // panic: can't use `+` on true and false
```

- **Spec 2.21:** a distinct type has "Nothing else: no arithmetic". **Spec 14.2:** `sum(f)` sums "each item's number".
- **What happens:** all accepted (`distinct3.kav`, `sum-any.kav`); the third panics. `n += NetId(1)` and `tot<T: Number>(xs)` are refused, so only `sum` leaks.
- **Where:** `builtin_methods.rs:147` and `more_methods.rs:239`: `"sum" => sig(vec![f(t, u.clone())], u)` with `u` a fresh variable of any kind. It should be bound to a number type (as a `Number` bound would be).

### 11. Contracts may call function values that use capabilities

```kavach
fn check(f: fn(Int) -> Int, x: Int) -> Int
    require f(x) > 0
{ x }
check(loud, 3)                           // loud prints; the require prints
```

- **Spec 7.5:** "It can only read values and call functions that have no `uses` clause." **Spec 8.1:** "Calling a function value needs nothing more."
- **What happens:** accepted, prints from the `require` (`contract-fnval.kav`).
- **Where:** `calls.rs:226` checks only the callee signature's declared `uses`; a call of a local of function type has none.
- **Spec conflict:** 8.1 lets any code call a function value for free; 7.5 says contracts are pure. Proposed rule: in a contract, a function value may be called only if its type has a `uses` limit that is empty (`fn(Int) -> Int uses ()` or similar syntax, to decide).

### 12. Interface bounds of generic types are never checked

```kavach
type Box<T: Describe> { item: T }
const a = Box(item: 5)                   // accepted
fn take(b: Box<Int>) -> Int { b.item }   // accepted
```

- **Spec 2.9:** as in finding 5. `Number` and `Comparable` bounds of types **are** checked (`type-bound.kav`, `type-bound2.kav`).
- **Where:** `collect.rs:155-172` records interface bounds (`self.bounds`) only for a function's generics. Type generics' interface bounds are never stored, so nothing checks them and nothing can use them (incompleteness I1 below). No conformance test, example or `stdlib/` type uses an interface bound on a type.
- **Harm today:** small, as the bound can't be used either; it becomes finding 5 the day I1 is fixed.

### 13. Type variables left open are accepted

```kavach
fn zero<T>() -> T? { none }
print(zero())                             // accepted: nothing says what T is
```

- **Spec 2.9:** type arguments are worked out from the arguments, "when they can't be, write them at the call"; `Box(raw: text)` with nothing saying what `I` is is a compile error.
- **What happens:** accepted (`unresolved.kav`).
- **Where:** `show.rs:117`, `default_vars`: a variable of kind `Any` left at the end becomes `Ty::Error` in the types handed to the interpreter and lowering, with no message. The [compiler review](compiler-review.md) found that lowering turns a missing type into `Unit`; these two meet here.

## Confirmed: refused but the spec allows it

- **I1. A type's interface bound can't be used.** `type Box<T: Describe> { item: T  fn show(self) -> String { self.item.describe() } }` gives "`T` has no method `describe`" (`bound-in-type.kav`); so does an invariant. Spec 2.9 allows bounds on a type's parameters, and spec 7.3 says an invariant "may use the type's parameters". Cause: `collect.rs:155-172`, as in finding 12.
- **I2. Tuple `match` coverage.** `match x { (true, _) => 1, (false, _) => 2 }` on a `(Bool, Bool)` is refused with "does not cover every other value" (`match3.kav`). Spec 4.3: "It must cover every possible case", and these arms do. The message on a truly incomplete tuple match names no missing case.
- **I3. One mistake, several messages.** `if c { [1, 2] } else { xs }` into `[Int]<max: 2>` reports the same mistake twice, at the `else` and at `xs` (`sized-if.kav`). `dot(xs, ys)` with plain lists reports three messages, two showing `len: _` (`numparam.kav`). This is the duplicate the compiler review could not measure.

## Suspected (from reading, not run)

- **S1. Polymorphic recursion never ends in native code.** `fn depth<T>(x: T, n: Int) -> Int { … depth([x], n - 1) }` is accepted and runs in the interpreter (`polyrec.kav`). KIR makes one instance per type argument list (`kir/src/lower_generic.rs:306`, the worklist at `lower.rs:247`) with no limit, so `kavach build` should loop until memory runs out. The spec says nothing; the checker should refuse a recursive call whose type arguments grow.
- **S2. `cli.parse<T>` and `ask<T>`** make records the way `json.decode` does, so they likely share finding 2.
- **S3. `ExprKind` on an unknown base gives `Ty::Error` silently:** `exprs.rs:202` (tuple field), `fields.rs:192` (index), `items.rs:461` (`for` item) and `patterns.rs:336` map `Ty::Var(_)` to `Ty::Error` with no message. Lambda bodies seem to be checked again once their parameter types are known (`var-error.kav` was refused), so no probe reached them; any other path where a base is still unknown would pass anything.
- **S4. `==` on function values** is accepted and gives `false` (`fneq.kav`). Spec 1.8 says `==` "compares contents" and says nothing of functions. An open question for the spec.
- **S5. No way to name an `inout` parameter's value before the call** in an `ensure` (spec 7.2: it sees the new value). `old(n)` is "not defined; did you mean `Add`?" (`contract4.kav`). An open question for chapter 7.
- **S6. Interface methods that declare `uses`** are not compared with their implementations (`conform.rs`), so even after a spec decision on finding 1, `conforms` needs the check.

## What held up

Probes of these were all refused, with good messages:

- **Frozen values (2.20):** passing to functions that keep, wrap, `match`-name or pass on a parameter; frozen into an interface value; frozen through `if`, `match`, `??`, a list literal, `map` and a generic identity (`frozen1.kav`, `frozen2.kav`, `frozen-if.kav`, `frozen-or.kav`).
- **Distinct types (2.21):** `==` with an `Int`, mixing two distinct types, `..`, held-type methods, `+=`, `T: Number` (`distinct.kav`, `distinct2.kav`). Only `sum` leaks (finding 10).
- **Optionals (2.7):** `if … else { none }` and a `let` set to `none` then returned as `Int` (`opt1.kav`).
- **Number parameters (2.9):** a parameter against a fixed `len`, plain lists into `len: n` (`numparam2.kav`, `numparam.kav`).
- **Capabilities** through default parameter values, module constants holding a function, `spawn`, operator and index methods (inferred `uses` included).
- **`match` on unions and optionals:** a guard doesn't count as covering; a missing `none` is reported (`match1.kav`, `match2.kav`).
- **Contracts:** direct `print` and `random` inside a lambda in a `require` or `ensure` (`contract2.kav`).

## 6. Where the rules are written down

**Today:**

- The **spec** is the source of truth for each rule, chapter by chapter. It is good at single rules and weak at their meeting points: 2.8 and 8.1 never say how interfaces and capabilities meet (finding 1), 7.5 and 8.1 disagree on calling function values (finding 11), 2.11 and 8.7 say "wherever their value goes" without listing the ways a value goes.
- The **checker README** lists 15 rules with one message each. It is out of date: it says "Not checked yet: capability inference and `uses` rules", but `capabilities.rs` does both.
- **631 compile-error conformance tests** pin one rule each. They test the direct case, almost never a value going through a method, a literal or a type parameter, which is where every hole above is.
- **The typing relation itself ("a `T` fits where a `U` is") is written nowhere.** It is spread over `infer.rs` (`unify`), `coerce.rs` (`coerce`), `size_checks.rs:110` (`coerce_sized`), `frozen.rs:213` (`coerce_frozen`), `fn_values.rs:31` (`fn_fits`) and `coerce.rs:71` (`implements_interface`). 48 call sites use `unify` directly and 20 use `coerce`, so which rules apply depends on which one a file happened to call.
- **Each tracked property is checked in many files:** frozen values at 96 sites in 20 files, capabilities at 35 sites in 14 files, secrets at 10 sites in 8 files.

**Proposal: one page, `docs/architecture/type-rules.md`, with two tables.**

1. **The "fits" relation:** one row per pair of type forms (`T` → `T?`, type → interface, `Frozen<T>` → `T`, sized → plain, `fn` with limit → `fn`, generic with and without arguments, `Never`, `Error`), what it requires, the spec section and the one function that decides it.
2. **The flow matrix:** rows are the ways a value moves (argument, result, field, default, list or dict item, tuple, branch, `??`, assignment, `op=`, method receiver, method result, interpolation, lambda capture, `spawn`, `json.decode`/`cli.parse`/`ask`); columns are the tracked properties (type, frozen, secret, capability, function-type limit, size, `pub(read)`/private). Each cell names the check, or says "not needed" and why.

Most findings above are an empty cell of that matrix: secret × method result, capability × default field, limit × list item, private field × `json.decode`. Filling the matrix is the review; a test per cell keeps it filled. In code, the matching change is one `fits(actual, expected, flow)` entry point that every flow calls, instead of `unify` at 48 sites.

## 7. A fuzzer for "accepted but should be refused"

Three oracles say a program the checker accepted is wrong. They can share the seeded, dependency-free generator of `compiler/driver/tests/fuzz.rs`.

**Oracle A: the interpreter's type confusions.** The interpreter already notices a wrong-typed value: `panic: expected an integer` (`interp/src/builtins.rs:262, 267`), `not a tuple` (`eval.rs:177`), `can't use + on …` (`ops.rs:295`). Today these look like ordinary panics. Tagged as internal, any of them in an accepted program is a checker bug, with no expected output needed. Findings 5, 6, 8, 9 and 10 each tripped one.

**Oracle B: indirections that must change nothing.** Take each of the 631 compile-error tests. On its `// error:` line, wrap the offending value in an indirection the spec says is transparent:

| Wrapper | Spec | Would have found |
|---|---|---|
| `id(x)` with `fn id<T>(x: T) -> T` | 2.9 | 5 |
| `(() => x)()`, `[x].map(v => v)[0]` | 3.4 | 6, 11 |
| `[x][0]`, `(x, 0).0`, `if true { x } else { x }` | 2.2, 4.1 | 3, 7 |
| `let t = x` then `t = x` | 8.7 | 3 |
| `x.trim()` / `x.upper()` on text | 8.7 | 3 |
| a call through an interface value or a bound | 2.15 | 1 |
| moved into a field default or an invariant | 2.4, 7.3 | 4 |

If the original is refused and the wrapped one is accepted, the wrapper found a hole. This is metamorphic testing; it needs no knowledge of what the program should print.

**Oracle C: run-time monitors for the static rules.** In the interpreter, behind a test switch:

- **Capabilities:** keep a stack of the capabilities each running function declares (or had inferred). A built-in that does I/O checks the stack up to the nearest function-value call (spec 8.1: a function value carries its maker's rights). A mismatch is a checker bug. This would have found 1, 4 and 11.
- **Secrets:** taint values read from a `secret` field, `env(…)` or a `-> secret` call; `reveal` clears it. A tainted value reaching `print`, a log or `ask` is a checker bug. This would have found 3.

Then run every conformance test, example and `stdlib/` test with the monitors on: they should be silent. Run the oracle B mutants with them too.

**Generators, in order of cost:**

1. **Mutate existing programs** (cheapest, highest yield): oracle B's wrappers on compile-error tests; and on passing tests, delete a `uses`, a `secret`, a bound, a `match` arm or a type argument, which must turn "accepted" into "refused".
2. **Grammar-based generation** of small well-typed programs from a typed template (pick a type, then an expression of it), as Csmith does for C. Costly to write, and best after the flow matrix exists, so each matrix cell gets generated cases.

**A concrete first step (about two days):**

1. In `compiler/interp`, give the three type-confusion panics one internal kind and a fixed prefix (`internal type error:`).
2. In `tools/cli/tests/conformance.rs`, fail any accepted program whose run hits that prefix.
3. Add a seeded test next to `driver/tests/fuzz.rs` that applies the first four wrappers of oracle B to the 631 compile-error tests' marked lines (a text edit of one line, kept only when the result parses), checks each, and lists those now accepted. Start it as a report, as the compiler review's cheap fix 2 does, to see how many there are before failing on them.
4. Turn each confirmed finding of this review into a conformance test (`expect compile error`) when its fix lands, per AGENTS.md section 5.

## Suggested order of fixes

| # | Fix | Size | Needs |
|---|---|---|---|
| 1 | `sum(f)` result must be a number (10) | small | nothing |
| 2 | `names.rs:133` reports an error (8); "no `Ty::Error` without an error" as a test | small | nothing |
| 3 | Bounds on `Ty::Param` and deferred `Ty::Var` (5, 6) | small | nothing |
| 4 | Bare generic type name in a signature is an error (9) | small | spec line in 2.9 |
| 5 | Open type variables are an error (13) | small | spec line in 2.9 |
| 6 | Needs of field defaults and invariants recorded (4) | medium | nothing |
| 7 | `json.decode`/`cli.parse`/`ask` refuse private and `pub(read)` fields outside the module (2) | medium | spec lines in 10.1, 2.12 |
| 8 | Secret flow over every expression and assignment (3) | medium | spec 8.7: list the flows, decide `len()` |
| 9 | Interfaces and capabilities (1, S6) | large | decision record: interface `uses` or pay at boxing |
| 10 | Function values in contracts (11) | small | spec decision 7.5 vs 8.1 |
| 11 | Limits compared inside types (7) | medium | the single `fits` entry point (section 6) |
| 12 | Type interface bounds stored and checked (12, I1) | medium | nothing |
| 13 | Polymorphic recursion refused (S1) | small | spec line in 2.9 |

## What this review could not check

- Nothing was built, so no probe ran as native code. Findings 5–9 and 13 reach lowering with wrong or missing types; what native code does with them (a lowering error, a crash, or a wrong result) is not known. S1 needs a `kavach build` to confirm.
- Agents, `limit` and `approve` rules, `ask<T>`, GPU functions and `unsafe` were not probed.
- Only the interpreter's panics were seen; a hole that gives a wrong value without a panic would not show in a probe that only prints.
- The probe programs are in the scratchpad, not in the repository. Each finding has its program inline above.
