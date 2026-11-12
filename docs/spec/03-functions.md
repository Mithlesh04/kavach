# 3. Functions

## 3.1 Declaring functions

```kavach
fn add(a: Int, b: Int) -> Int {
    a + b
}
```

- The last expression is the return value.
- `return` exits early.
- Parameters can't be reassigned, like `const` (1.4). To change a value, copy it: `let total = start`.
- There are no rest parameters (`...args` in JavaScript, `*args` in Python): a function that takes any number of values takes a list, as in Rust, Zig and Haskell. `fn total(prices: [Int])` is called `total([4, 8, 15])`; `total(4, 8, 15)` is a compile error that says to put the values in a list.
- A function with no `->` returns nothing, and a call to it can't be used as a value.
- So a function without `->` must not end with a value. The compiler shows the fix:

```
error: double has no `->`, so it returns nothing, but its last line is a value
  fn double(x: Int) { x * 2 }
                      ^^^^^ this Int would be thrown away
  fix: fn double(x: Int) -> Int { x * 2 }
```

```kavach
fn grade(score: Int) -> String {
    if score < 0 {
        return "invalid"
    }
    if score >= 90 { "A" } else { "B" }
}
```

## 3.2 Named arguments and defaults

Any parameter can be passed by name. Positional arguments come first.

```kavach
fn greet(name: String, greeting: String = "Hello") -> String {
    "{greeting}, {name}!"
}

greet("Asha")                          // "Hello, Asha!"
greet("Asha", greeting: "Namaste")     // "Namaste, Asha!"
```

## 3.3 Lambdas

```kavach
const double = x => x * 2
const add = (a, b) => a + b
const log = msg => {
    debug(msg)
    msg.len()
}
```

Lambda parameter types are inferred from where the lambda is used.

A lambda keeps a **copy** of each outside value it uses, taken when the lambda is created. Changing the variable later doesn't change the lambda, because values are never shared (6.1). To share a changing value, use `Shared<T>` (6.2).

```kavach
let limit = 10
const over = n => n > limit
limit = 0
over(5)          // false: the lambda still sees 10
```

A lambda can't change an outside variable either. Changing its copy, with `=`, `+=` or a method that changes a value (`push`, `add`, `remove`), lasts only for that call: each call starts again from the copy taken when the lambda was made. The compiler warns, since the change is almost always a mistake:

```kavach
let total = 0
const sums = [1, 2, 3].map(x => {
    total += x       // warning: `total` is a copy inside this lambda
    total
})
// sums is [1, 2, 3], and total is still 0
```

A `try` or `fail` inside a lambda belongs to the lambda: it ends that call of the lambda with the error, and the lambda is a function that can fail (3.4). The error never jumps out of the function the lambda was written in, since the lambda may be called long after that function has returned ([decision 0085](../decisions/0085-function-types-with-errors-and-uses.md)). A built-in method given such a lambda passes its error on (5.2):

```kavach
const prices = try orders.map(o => try price(o))    // the first order that fails stops `map`
```

### Dot shorthand

Assigning to an outside variable inside a lambda changes only the lambda's copy, so the variable outside stays as it was. That is rarely what was meant, so it gives a warning, and the same inside `spawn { … }` (9.5):

```kavach
let count = 0
const items = [1, 2, 3].parallel_map(n => {
    count += 1              // warning: `count` is a copy inside this lambda
    n
})
print(count)                // 0
```

Give the value back instead (`items.len()` here), or use `Shared<T>` (6.2) when tasks must change the same thing.

In an argument, an expression starting with `.` is a lambda over one value:

```kavach
orders.filter(.status == Status.Paid)       // same as: orders.filter(o => o.status == Status.Paid)
orders.sum(.amount)                  // same as: orders.sum(o => o.amount)
names.map(.upper())                  // same as: names.map(n => n.upper())
```

## 3.4 Functions are values

```kavach
const op: fn(Int, Int) -> Int = add
const result = op(2, 3)
```

A generic function (2.9) used as a value takes its types from where it is used: `const ints: fn([Int]) -> Int? = first`.

A function type says what its functions can fail with and what they may use, in the order of a signature (3.5): `fn(NewOrder) -> Order ! ApiError uses Db`. Several capabilities go in parentheses, `uses (Net, Db)`, so that a `,` after the type still ends it ([decision 0085](../decisions/0085-function-types-with-errors-and-uses.md)).

- A function that can fail fits only a function type that says so, with an error its own fits (5.2). A function that can't fail fits any.
- Calling a function value that can fail is a call that can fail: handle it with `try`, `catch` or `match` (chapter 5).
- The error can be a type parameter, so generic code keeps the error type of the function it is given, and a `match` on it stays exhaustive.
- With `uses`, a function value may use only what its type lists. Without it, the type sets no limit. Either way, the code that makes the value, by naming a function or writing a lambda, must have what the function uses (8.1).

```kavach
type ApiError = OutOfStock(item: String) | Bad(reason: String)

fn create(o: NewOrder) -> Order ! ApiError uses Db { … }

fn route<P, R, E>(input: P, handler: fn(P) -> R ! E) -> R ! E {
    try handler(input)
}

const strict: fn(NewOrder) -> Order = create               // error: `create` can fail with `ApiError`
const loose: fn(NewOrder) -> Order ! ApiError uses Fs = create   // error: `create` uses `Db`, and this type allows only `Fs`
const order = route(new_order, create) catch e => match e {
    ApiError.OutOfStock(item) => …,
    ApiError.Bad(reason) => …,
}
```

A record field holding a function is called like a method: with `type Handler { on: fn(Int) -> Int }`, both `h.on(x)` and `(h.on)(x)` call the field's function. A method of the same name on the type wins over the field.

## 3.5 The full signature

Every part after the parameters is optional, but when present, the order is always the same:

```kavach
pub fn transfer(from: Account, to: Account, amount: Decimal)   // name and parameters
    -> (Account, Account)                                      // return type
    ! TransferError                                            // errors it can return (chapter 5)
    uses Payments                                              // capabilities it needs (chapter 8)
    require amount > 0                                         // contracts (chapter 7)
    ensure result.0.balance == from.balance - amount
    example transfer(alice, bob, 10).1.balance == bob.balance + 10   // examples (7.6)
{
    …
}
```

Reading only the signature tells you what a function takes, returns, can fail with, can touch, promises, and how it behaves on a real input. That is what people review when AI writes the body.

## 3.6 Visibility

Functions, types and fields are private to their module unless marked `pub` (chapter 10).

## 3.7 Using results

A call that returns a value must use it. To throw a value away on purpose, write `const _ =`:

```kavach
add(1, 2)              // error: the result of add is unused
const _ = add(1, 2)      // fine: clearly on purpose
print("done")          // fine: print returns nothing
```

A forgotten result is a common bug, for example making an updated copy with `order with { … }` and never using it. Errors are stricter still: they must be handled (chapter 5).

## 3.8 Recursion

Functions may call themselves. Tail calls are **not** guaranteed to be optimized, so very deep recursion can run out of stack. That is a panic (5.6) with a clear message, never memory corruption. For long loops, use `for` or `while`.

## 3.9 inout parameters

A parameter marked `inout` lets a function change the caller's variable ([decision 0038](../decisions/0038-inout-parameters.md)):

```kavach
fn bump(inout n: Int) {
    n += 1
}

fn swap(inout a: Int, inout b: Int) {
    const t = a
    a = b
    b = t
}

let count = 0
bump(count)                    // count is now 1
let x = 1
let y = 2
swap(x, y)                     // x is 2, y is 1
```

- Inside the function, an `inout` parameter is a variable: it can be assigned and changed (`n += 1`, `items.push(4)`).
- The argument is a place the caller could assign to: a `let` variable, or a field or list item of one (`bump(user.age)`, `bump(scores[0])`). A `const`, a value or a call is a compile error.
- When the function returns, the parameter's last value goes back to that place. Nothing is shared while the function runs: it changes its own copy, so no other name sees half a change. When the function fails (chapter 5), the place keeps its old value.
- The same variable can't be given twice in one call when one of them is `inout` (`swap(x, x)`, or `bump(x)` beside another argument naming `x`): that would be two names for one value, and is a compile error.
- An `inout` parameter is itself a place: a function can pass it on as `inout` to another call, a recursive one too, and every change reaches the first caller's variable:

  ```kavach
  fn count_down(n: Int, inout total: Int) {
      if n == 0 {
          return
      }
      total += n
      count_down(n - 1, total)
  }

  let total = 0
  count_down(4, total)           // total is now 10
  ```

- A lambda can't change an outside variable, an `inout` parameter included: it keeps a copy (3.3), so a change inside it doesn't reach the outside, and the compiler warns. Have the lambda give back a value (`total += items.sum(x => x)`), or use `Shared<T>` (6.2).
- A function with `inout` parameters can't be used as a function value (3.4). So a lambda, which is one, can't have `inout` parameters: give the function a name.
- An interface method can have `inout` parameters. Each type that implements it marks the same parameters `inout` (2.8), so a call through an interface value (2.15) changes the caller's variable too.

## Open questions

None right now. Decisions are recorded in [0013](../decisions/0013-core-language-decisions.md).
