# 6. Memory

Memory is **automatic by default** and gives you **more control when you ask for it**. There is no garbage collector and no pause.

| Level | You write | Use for |
|---|---|---|
| 1. Automatic | nothing | Almost all code |
| 2. Shared state | `Shared<T>`, `Weak<T>` | State that several parts of the program change |
| 3. Arenas | `arena { … }` | Hot loops, parsers, request handling: many short-lived values |
| 4. Unsafe | `unsafe { … }` | Hardware, C interop, hand-tuned data structures |

## 6.1 Level 1: automatic

### Values, not references

Every type is a value. Assigning or passing a value never makes two variables share it:

```kavach
let a = [1, 2, 3]
let b = a
b.push(4)
// a is still [1, 2, 3]
```

This removes a whole class of bugs ("who else changed my list?") and makes concurrency safe (chapter 9).

### How it stays fast

Copying everything would be slow, so the compiler avoids it:

1. **Ownership is inferred.** When a value is used for the last time, it is *moved* instead of copied.
2. **Reference counting under the hood.** Large values are shared internally and counted. The compiler removes most count updates at compile time.
3. **Copy-on-write, reused in place.** A value is copied only when it is changed *while* someone else still holds it. If nobody else holds it, it is changed in place. So `text += part` or `items.push(x)` in a loop copies only what is added, not the whole value each time.

You never see any of this. You write simple value code and get performance close to hand-managed memory.

What is counted, as the compiler does it today (a program can't tell the difference):

| Values | Where they live |
|---|---|
| Numbers, `Bool`, `Char` | In place: in a variable, or inside the value that holds them. Never counted, never freed on their own |
| Text, `Bytes`, `Decimal`, `BigInt`, lists, maps, sets, records, tuples, unions, optionals of these, functions, interface values, `Shared`, tasks, channels, errors | One object each, which counts its holders and is freed when the count reaches zero |

A holder is anything that keeps a value: a variable, a list or record it is inside, a lambda that uses it (3.3), a task. A record a function never shares may never become an object at all: its fields stay in place, like numbers.

### Freed immediately, predictably

A value is freed the moment its last use ends. Types that hold resources can run cleanup by implementing `Drop`:

```kavach
type TempFile: Drop {
    path: String

    fn drop(self) uses Fs {
        remove_file(self.path) catch _ => {}
    }
}
```

`drop` may need capabilities, like any function. They count as needed wherever a value of the type is **created**, so `TempFile(path: p)` requires `Fs`. Code that only receives a `TempFile` needs nothing extra.

**When `drop` runs.** A value of a type that implements `Drop`, or that holds one (a record, tuple or list with such a value inside), lives until the end of the block that holds it, even after its last use. So a guard works as written:

```kavach
type Guard: Drop {
    name: String

    fn drop(self) uses Console {
        print("released {self.name}")
    }
}

fn work() uses Console {
    const g = Guard(name: "data")   // never read again
    print("working")                // still guarded
}                                   // prints "working", then "released data"
```

- At the end of a block, the values it made drop in the reverse order they were made: the last made drops first.
- `return`, `break`, `continue` and `fail` end the blocks they leave: those blocks' values drop right then, in the same order.
- A generic type can implement `Drop` too (`type Box<T>: Drop`): every instance (`Box<Int>`, `Box<String>`) runs the same `drop`. So can a union (`type Lock: Drop = Held(name: String) | Free`), with `drop` in an `extend`: it runs for every variant.
- A value made without a name (a temporary, as in `print(Door(name: "x").name)`) drops at the end of its statement. So does one given to `_` (`const _ = Guard(…)`), since `_` names nothing; a warning says so, as this is almost always meant to keep the value: name it `_guard` instead.
- Giving a variable a new value drops the old one there, unless something else still holds it.
- Copies are the same value (6.1): `drop` runs once, when the last holder's block ends. A value returned from a function is held by the caller. Changing a copy makes it a value of its own, which drops on its own.
- A lambda that uses a `Drop` value holds a copy (3.3), and so does an interface value made from one (2.15): each lives, like any holder, to the end of the block that holds it.
- An `inout` parameter is a copy too (3.9): while the call runs, the caller's variable still holds the old value. When the function gives the parameter a new value, the old one drops when the call returns.
- After `drop` returns, the value's fields are freed. `drop` runs once per value, in every way of running a program (`kavach run`, `run --native`, `kavach build`).
- A panic runs no `drop`. In the main task it ends the program (5.8); in another task, or in an agent's handler, it stops just that, and what it held is never dropped ([decision 0065](../decisions/0065-a-panic-stops-only-its-task.md)).

Other values are still freed the moment their last use ends: nothing can tell. (A `Shared` lives to the end of its block too, since a `Weak` can tell, 6.2; and so do a channel's ends, since a sender can tell when every receiver is gone, 9.4.)

Files, sockets and locks close at a known point. There is no "eventually, when the collector runs".

**No `delete`.** There is no `delete` (C++), `del` (Python) or `= null` (JavaScript) to free a variable's memory. The compiler already frees a value at the earliest safe moment, and a `delete` could only add a use after it. To free memory sooner, stop using the value, give the variable a new value, or end the block that holds it:

```kavach
let rows = load_rows()          // a large list
const total = rows.len()
rows = []                       // the old list is freed here
```

An arena (6.3) frees many values at once; `free` inside `unsafe` (6.4) gives back raw memory, as in C.

## 6.2 Level 2: shared state

When several parts of a program must see and change **the same** thing, say so explicitly:

```kavach
const counter = Shared(0)

counter.update(n => n + 1)      // safe from any task; locking is automatic
const now = counter.get()
```

- `Shared<T>` is the only way to get shared, mutable state. Seeing `Shared` in code tells readers "this is shared".
- `update` holds the value's lock while its function runs, so updates from many tasks, on any number of cores, never lose one ([decision 0052](../decisions/0052-shared-update-holds-its-lock-and-never-waits.md)). The function may not wait: no channel `put` or `take`, no `.wait()`, no `sleep`, no network or file call, no `ask`. A wait in the function itself is a compile error; one reached through another function panics. Get what the update needs first:

```kavach
const page = try fetch(url)
cache.update(c => c.with(url, page))     // not: cache.update(c => c.with(url, try fetch(url)))
```
- `update` gives the new value, read while the lock is still held; it may be left unused (3.7). `update_with` takes a function that gives a pair, the new value and a result, and gives the result: take something and know what you took, in one step ([decision 0095](../decisions/0095-shared-update-gives-a-result.md)):

```kavach
const next_id = Shared(1)
const id = next_id.update_with(n => (n + 1, n))     // 1, and next_id is 2
const total = counter.update(n => n + 1)            // the count now
```
- `Weak<T>` points to a `Shared<T>` without keeping it alive. Use it for back-links (child → parent) so no cycle forms. `s.weak()` makes one; `w.shared()` gives the `Shared<T>` back while it lives, and `none` once it was freed:

```kavach
type Door {
    to: Weak<Room>            // a door doesn't keep its room alive
}

const hall = Shared(Room(name: "hall", doors: Shared(no_doors)))
const back = hall.weak()
if let room = back.shared() {
    print(room.get().name)    // hall
}
print(Shared(5).weak().shared() == none)   // true: nothing held the 5
```

- A `Shared`, or a value that holds one, lives until the end of the block that holds it, as a `Drop` value does (6.1). So what `w.shared()` gives depends on the blocks, as written, not on where a `Shared` was last used.
- Cycles can only form through `Shared`. There is no cycle collector: a leaked cycle is a bug to fix, and the report points straight to it. `kavach run` and `kavach test` report each `Shared` still alive when the program ends, which only a cycle can keep, with the line that made it; a native program does the same when run with `KAVACH_CHECK_LEAKS=1`, and otherwise pays nothing for it:

```text
leak: a `Shared` value made at rooms.kav:8 was never freed: a cycle keeps it
  make the link back a `Weak`, so no cycle forms (spec 6.2)
```

## 6.3 Level 3: arenas

An arena allocates values quickly from one block of memory and frees them all at once at the end:

```kavach
fn parse_all(lines: [String]) -> Summary {
    arena {
        const tokens = lines.flat_map(tokenize)       // thousands of small values
        summarize(tokens)                           // the result is copied out
    }                                               // everything else is freed here, at once
}
```

- Values created inside an arena **must not escape** it. The compiler checks this.
- The only way out is the block's value. Storing an arena value anywhere outside (a variable, list or `Shared` declared before the arena) is a compile error. Return it as part of the block's value instead.
- Tasks spawned inside an arena may use its values. A task never outlives the code that spawned it (chapter 9), so it always finishes before the arena ends.
- The value of the arena block is copied out automatically.
- Use an arena when a profiler shows allocation is a bottleneck, not by default.

## 6.4 Level 4: unsafe

`unsafe` turns off safety checks for a block so you can work with raw memory:

```kavach
unsafe {
    const p: Ptr<UInt8> = alloc(1024)
    p.write(0, 255)
    free(p)
}
```

Inside `unsafe` you can use `Ptr<T>`, `alloc`, `free`, raw pointer arithmetic and calls to C.

A `Ptr<T>` is the address of `T` values in memory Kavach doesn't manage, as C has it. `T` is a number, a `Bool` or a `Ptr`, the types C passes as they are (10.7), or a record of only those. Such a record is laid out as C lays out a `struct` with the same fields in the same order: each field at the next offset that is a multiple of its size, and the whole padded to a multiple of its largest field. A record read through a `Ptr` is checked against its `invariant`s, as a new record is (7.3).

```kavach
type Point {
    x: Int32
    y: Int32
}

unsafe {
    const ps: Ptr<Point> = alloc(2)         // 2 × 8 bytes, as C's struct { int32_t x, y; }
    ps.write(1, Point(x: 3, y: 4))
    print(ps.read(1).y)                   // 4
    free(ps)
}
```

| Operation | Does |
|---|---|
| `alloc(n)` | `n` values of `T`, set to zero, from C's `calloc`; the type comes from where the result goes (`const p: Ptr<Int32> = alloc(4)`) |
| `free(p)` | Gives the memory back (C's `free`), so C code may free what Kavach allocated and the other way round |
| `p.read(i)` | The `i`th value (from 0) |
| `p.write(i, v)` | Sets the `i`th value to `v` |
| `p.text()` | For a `Ptr<UInt8>`: a copy of the text there, up to the first zero byte, as C keeps text; `none` for a null pointer; bytes that aren't UTF-8 panic |
| `p.bytes(n)` | For a `Ptr<UInt8>`: a copy of the `n` bytes there, as `Bytes`; `.text()` on them gives `none` for bytes that aren't UTF-8, where `p.text()` panics |
| `Ptr.from_int(n)` | The address `n`, for values C defines as numbers cast to pointers (SQLite's `SQLITE_TRANSIENT` is `-1`) |

C's pointer syntax works too, inside `unsafe`, for the same operations:

| Written | Is |
|---|---|
| `*p` | `p.read(0)`: the value `p` points at |
| `*p = v` | `p.write(0, v)`; `*p += 1` and the other changing assignments too |
| `p[i]` | `p.read(i)` |
| `p[i] = v` | `p.write(i, v)` |
| `p + n`, `p - n` | The address `n` values of `T` further on, or back: a `Ptr<T>` |

```kavach
unsafe {
    const p: Ptr<Int32> = alloc(3)
    *p = 7
    p[1] = 8
    const q = p + 2
    *q = *p + p[1]
    print(p[2])                   // 15
    free(p)
}
```

Nothing is checked: an index past the end, a pointer used after `free`, or freeing twice is undefined behavior, as in C. That is why each of these is allowed only inside `unsafe { … }`. Pointers run in native code; the interpreter stops at the first one.

Unsafe code is **opt-in per package** and always visible:

- A package must set `unsafe = true` in `kavach.toml` to use `unsafe` at all.
- `kavach audit` lists every package in your dependency tree that uses unsafe code.
- Wrap unsafe code in a small safe function and keep it in one place.

## Open questions

None right now. Decisions are recorded in [0013](../decisions/0013-core-language-decisions.md).
