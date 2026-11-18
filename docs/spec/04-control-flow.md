# 4. Control Flow

## 4.1 if / else

`if` is an expression. Conditions must be `Bool`; there is no "truthiness".

A condition written with `=` (`if x = 4`) is a compile error that says to write `==`: `=` sets a variable, and only as a statement (1.8). The same holds for `while`.

```kavach
if stock == 0 {
    print("Sold out")
} else if stock < 5 {
    print("Only {stock} left")
} else {
    print("In stock")
}

const label = if paid { "paid" } else { "due" }
```

## 4.2 if let

Runs a block only when an optional has a value:

```kavach
if let user = find_user(id) {
    print(user.name)
} else {
    print("No such user")
}
```

## 4.3 match

`match` compares a value against patterns, top to bottom. It must cover every possible case, so adding a new variant to a union shows you every place to update.

```kavach
const text = match shape {
    Shape.Circle(radius) => "circle of radius {radius}"
    Shape.Rect(width, height) if width == height => "square of side {width}"
    Shape.Rect(width, height) => "{width} by {height} rectangle"
}
```

Patterns:

| Pattern | Matches |
|---|---|
| `42`, `"ok"`, `true` | That exact value |
| `Status.Paid` | A union variant, always with its union's name (2.6) |
| `Shape.Circle(radius)` | A variant, binding its field |
| `Shape.Rect(width: w, height: _)` | Binding a field under another name, ignoring one |
| `1..=9` | A value in a range |
| `(0, y)` | A tuple |
| `none` | A missing optional |
| `ok(value)` / `err(e)` | Success or failure of a call that can fail (chapter 5) |
| `_` | Anything |
| `pattern if condition` | A pattern plus an extra check (a guard) |

Covering every case looks inside nested variants: arms that differ in one field cover the variant when, between them, they cover that field's type. A `match` on an optional may list `none` and the variants of the value inside, which match a present value:

```kavach
match parcel {                                   // parcel: Box, a `Full(s: Shape)` or `Empty`
    Box.Full(s: Shape.Circle(radius)) => …,
    Box.Full(s: Shape.Square(side)) => …,        // with the arm above, covers every `Box.Full`
    Box.Empty => …,
}

match found {                                    // found: Shape?
    Shape.Circle(radius) => …,                   // a present circle
    Shape.Square(side) => …,
    none => …,
}
```

An arm may leave instead of giving a value: `return`, `fail`, `break` or `continue`. The `match` then has the value of the arm that was taken, and the code after it runs only when one that gives a value was:

```kavach
const amount = match share {
    Share.Full => price,
    Share.Half => price / 2,
    Share.Escalate => return "escalate",    // leaves the function
}
```

## 4.4 for

```kavach
for name in names {
    print(name)
}

for i in 0..10 {
    print(i)            // 0 to 9
}

for (name, age) in ages {      // in insertion order (2.2)
    print("{name} is {age}")
}

for (index, name) in names.enumerate() {
    print("{index}: {name}")
}
```

The loop's name (`name`, `(index, name)`) is a `const` for each turn: assigning to it is a compile error, as are names taken by a `match` or `catch` arm and a lambda's parameters. A `for` walks the list as it was when the loop began: items pushed while it runs are not visited (`for x in xs { xs.push(x * 10) }` turns `[1, 2]` into `[1, 2, 10, 20]`).

A loop over `0..=xs.len()` that reads `xs[i]` gets a warning: its last turn reads `xs[xs.len()]`, one past the last item, and panics. Write `0..xs.len()`, or `for x in xs`. A loop that changes `xs`, or reads it at another index, gets none.

```kavach
for i in 0..=items.len() {     // warning: `i` reaches `items.len()`, one past the last index
    print(items[i])
}
```

`_` discards the value, to repeat without using it:

```kavach
for _ in 0..3 {
    knock()
}
```

## 4.5 while

```kavach
let attempts = 0
while attempts < 3 {
    attempts += 1
}
```

Use `while true { … }` for an endless loop.

## 4.6 break and continue

`break` leaves the innermost loop. `continue` skips to its next iteration.

A loop may have a label, a name and a colon before `for` or `while`, and `break` and `continue` may name it, to leave or go on with an outer loop ([decision 0054](../decisions/0054-loops-can-have-labels.md)):

```kavach
outer: for row in grid {
    for cell in row {
        if cell < 0 {
            continue outer       // the next row
        }
        if cell == target {
            break outer          // leaves both loops
        }
    }
}
```

- A label names a loop around the `break` or `continue`, in the same function: a lambda's body is a function of its own, so a `break` inside one can't reach a loop outside it.
- Naming a label that isn't around it is a compile error that lists the ones that are. Two loops around each other can't share a label.

## 4.7 Code that never runs

Code that can never run is almost always a mistake, made under pressure and found much later. The compiler warns about three kinds, each with what to change:

- **A `match` arm that can't match**: it comes after an arm that matches everything (`_`, or a name), or its pattern is the same as an earlier arm's. The earlier arm always wins.
- **An `else if` that can't be true** when the branches before it are false: its condition compares the same variable with a number in a way an earlier condition already covers (`x > 20` after `x > 10`), or repeats an earlier condition.
- **A `while` loop that never ends**: its condition reads only variables the loop never changes, and nothing inside leaves it (`break`, `return`, `fail` or `?`). `while true { … }` says an endless loop is meant, so it gets no warning.

The arm and the branch have one right fix: take them out. The editor's quick fix does it for one, and `kavach fix <files or folders>` for all of them (10.4).

```kavach
if score > 10 {
    print("good")
} else if score > 50 {      // warning: never true: `score > 50` means `score > 10`, taken above
    print("great")
}

let n = 0
while n < 10 {              // warning: never ends: `n` doesn't change in the loop
    print(n)
}
```

## 4.8 Blocks given to a call

A few standard library functions run a block inside something they set up, and end it however the block ends ([decision 0107](../decisions/0107-traces-and-metrics.md)): `log.span("checkout") { … }` runs the block as one span of a trace (14.12), and `random.seeded(42) { … }` draws the block's random numbers from a seed (14.15).

```kavach
const total = log.span("checkout") {
    const cart = try load_cart(user)
    cart.sum(item => item.price)
}
```

- The block is a plain block, not a lambda (3.3): `try`, `return`, `break` and `continue` act as they would without the call, and it can change the variables around it.
- The expression's value is the block's value.
- What the function set up ends when the block ends: by its value, an error, a `return`, a `break` or a panic.
- The block starts on the same line as the call's `)`.
- Only the standard library functions that say so take a block. A block after any other call is a compile error, and so is one of these calls without its block.

## 4.9 Where a loop runs

Every loop runs on one core, in order, as in C, Go, Rust and JavaScript. `on` before one `for` says where its rounds run instead ([decision 0126](../decisions/0126-loops-on-one-core-unless-on-cores.md)):

```kavach
for i in 0..n { out[i] = f(i) }                 // one core, in order
on cores for i in 0..n { out[i] = f(i) }        // all the program's cores
on cores(2) for i in 0..n { out[i] = f(i) }     // at most 2 of them
on threads(8) for i in 0..paths.len() {         // waiting work: up to 8 rounds wait at once
    sizes[i] = fs.size(paths[i])
}
rows: on cores for row in 0..h { … }            // a label comes first
```

- **`on cores` gives the one-core result.** Each round may write only its own slot of a list (`out[i]`, where `i` is the loop's name) and add whole numbers into a `let`: `total += n`, `total = total + n`, or a sum kept modulo a number written in the code, `total = (total + n) % 1_000_000_007` (native code spreads that one only where it proves no part is negative, so the order can't change it). Anything else in its body is a compile error that says why: `print` (its order would change), I/O, other changes outside the round, a float `+=` (the adding order changes the bits). What it reads is unrestricted.
- **`on threads(n)` is for waiting work:** its rounds may do I/O, in any order between rounds, and each round's result goes to its own slot.
- **It covers that loop only.** A loop nested inside runs on one core in each round unless it has its own `on`; when both levels do, they share the cores.
- **Leaving early works as on one core.** `continue`, `break`, labels, `return`, `fail` and `?` are allowed: the first round in order that leaves decides, and rounds after it change nothing.
- **How many cores:** `cores` is all the program may use (`[run] cores` in `kavach.toml`, or `KAVACH_CORES`, which wins when both are set; all of the machine's by default, never more than a container or CPU affinity allows). `cores(n)` asks for at most `n`; more than exist uses all it may. A count of 0 or less is a compile error when written as a number, a panic when worked out while running. `threads(n)` may exceed the cores, as its rounds mostly wait, up to `[run] threads` (256 by default) in all.
- `while` can't take `on`: each round depends on the one before. `on`, `cores` and `threads` are names everywhere else.
- `on` also goes before a value, for library work and the GPU (15.2): `on cores(1) numbers.sort()`, `on gpu(1) particles.map(step)`, `on cpu particles.map(step)`.

## Open questions

None right now. Decisions are recorded in [0013](../decisions/0013-core-language-decisions.md).
