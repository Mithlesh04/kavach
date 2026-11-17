# 0126. Loops on one core unless `on cores` says otherwise

- **Status:** Accepted
- **Date:** 2026-11-17

## Context

Decision [0031](0031-loops-on-all-cores.md) let the native compiler spread a math loop over all cores on its own, whenever it could prove the result equals the plain loop's. Nothing in the code showed which loops did this. A maintainer asked what a developer does who wants a loop on one core: to leave cores free for other programs, to save battery, to keep timing steady, or to measure one core. The only answer was `KAVACH_SERIAL=1`, a debugging switch for the whole program.

Other languages run a loop on one core unless the code asks for more: C, Go, Rust and JavaScript all do. Code that reads the same in Kavach should cost the same.

## Decision

Every loop runs on one core, in order. A prefix before one statement says where it runs instead (a maintainer confirmed this design on 2026-11-17, after several rounds of options):

```kavach
for i in 0..n { out[i] = f(i) }                 // one core, in order (default)
on cores for i in 0..n { out[i] = f(i) }        // all the program's cores
on cores(2) for i in 0..n { out[i] = f(i) }     // at most 2 of them
on threads(8) for i in 0..paths.len() {         // waiting work: up to 8 rounds wait at once
    sizes[i] = fs.size(paths[i])
}

const b = on gpu(1) particles.map(step)         // GPU 1, this call only
const c = on cpu particles.map(shade)           // the CPU, this call only
const sorted = on cores(1) numbers.sort()       // library work on fewer cores

gpu fn step(p: Particle) -> Particle { … }      // kept: the system picks a GPU, or the CPU
gpu(1) fn shade(px: Float32) -> Float32 { … }   // new: this function's maps on GPU 1

spawn fetch(url)                                // kept
spawn cores crunch(chunk)                       // kept: tasks share the cores, no count
spawn thread read_file(path)                    // kept
```

- **`on` covers one statement.** It applies only to the `for` or the value right after it. Nested loops decide for themselves, and when several levels use cores they share the cap. A label comes first: `rows: on cores for …`. `while` can't take `on`, as each round depends on the one before.
- **An `on cores` loop gives the one-core result.** Each round writes only its own slot (`out[i]`), or adds whole numbers into a `let`. The compiler checks this, and anything else is a compile error that says why: `print` (its order would change), I/O, other changes outside the round, a float `+=` (adding order changes the bits).
- **Leaving early works as on one core.** `continue`, `break`, labels, `return`, `fail` and `?` are allowed: the first round in order that leaves decides, and rounds after it change nothing. A loop that can leave early keeps the slots it writes until the order is known.
- **`on threads(n)` is for waiting work.** Its rounds may do I/O, in any order between rounds, and each round's result goes to its own slot.
- **One cap for the program:** how many cores it may use, all by default. `[run] cores = N` in `kavach.toml` sets it, or `KAVACH_CORES` at launch. Waiting threads don't count against it. `[run] gpu = cpu` (or `KAVACH_GPU=cpu`) keeps every map on the CPU.
- **Library work whose result can't differ uses the program's cores,** as NumPy and BLAS do: a big `sort`, a matrix `@`. So do `http.serve` and `grpc.serve`, and `ServeLimits(cores: n)` keeps one server's handlers on fewer.
- **`gpu(n) fn`** fixes a function's GPU, numbered as `system.gpus()` lists them. A program asking for a GPU the machine doesn't have stops at start, saying which.
- `on`, `cores`, `threads`, `gpu` and `cpu` stay ordinary names everywhere else.

**Edge cases** (a maintainer confirmed these on 2026-11-17):

| Case | What happens |
|---|---|
| The code asks for more cores than exist (`on cores(11)` on 10) | It uses all it may, with no error: the same program runs on many machines. `KAVACH_PAR_LOG=1` prints how many it used |
| `[run] cores = 11` on a 10-core machine | The cap is 10; the log notes it at start |
| Only 5 of 10 cores are free, as other programs are busy | At most 10 workers still start, but the loop is cut into many small chunks that workers take as they finish: a busy core takes fewer, so the loop never waits on one slow core. Kavach can't reserve cores; the system shares them |
| A container or CPU limit (`docker --cpus=2`, CPU affinity) | The machine's cores are the ones the program may use: the cap is 2 |
| `on cores(0)` or a negative count | A literal is a compile error; a value worked out while running is a contract panic |
| `on cores` inside `on cores` | Inner and outer loops share the workers, never more than the cap |
| 10,000 `spawn cores` tasks | All run, taking turns on the workers |
| `on threads(n)` and `spawn thread` | Threads may outnumber cores, as they mostly wait. `[run] threads` (default 256) limits them in all; past it, a new one waits for a free one |
| Child processes (`std.process`) | Not counted: each is its own program. A Kavach child inherits `KAVACH_CORES`, so a parent passes its cap on |
| `gpu(3) fn` on a machine with 2 GPUs | The program stops at start, naming the missing GPU |
| `on gpu(1)` while another program uses that GPU | It runs; the driver shares the GPU |

## Consequences

- A developer sees in the code which loops use several cores, and every other loop costs what it would in C.
- `kir/par.rs` keeps its proof that a loop's rounds are independent. It now runs only on loops marked `on cores`, and reports its findings as compile errors instead of silently falling back.
- Benchmarks that relied on automatic splitting must add `on cores` to keep their numbers.
- `KAVACH_SERIAL` goes; `KAVACH_CORES=1` does the same, for the whole program.
- The parser, the checker, `runtime/gpu` (device choice), spec 4, 9.8, 15 and 10's settings, the editor grammar and the docs site all change.

## Alternatives considered

- **Keep automatic splitting, add a way to turn it off:** fastest by default, but which loops split stays invisible, and it differs from every mainstream language.
- **`parallel for`, `forall`, a `parallel { }` block, or methods such as `parallel_each`:** each was shown with examples. A block suggests that everything inside it runs in parallel, which isn't what happens, and a new loop keyword adds a word that `spawn cores` already says.
- **A `with` clause (`with cores for …`):** reads well, and Kavach already uses `with` for settings in tests. `on` reads as plain English ("on cores, for each i") and can't be confused with a record copy (`x with { … }`).
- **A count with `=` (`cores = 2 for …`):** looks like an assignment wherever a program also has a variable named `cores`.
