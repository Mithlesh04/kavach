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

## Open questions

None right now. Decisions are recorded in [0013](../decisions/0013-core-language-decisions.md).
