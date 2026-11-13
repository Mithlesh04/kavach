# compiler/interp/src/testing

Generated tests for `kavach test` (spec [chapter 11](../../../../docs/spec/11-testing.md)): every function is called with generated arguments that meet its `require`; a panic, a broken `ensure` or a broken invariant is a failure.

| File | Job |
|---|---|
| `mod.rs` | The run: found cases first, generated cases, the report |
| `targets.rs` | Which functions are called and how their arguments are made; the others listed as not generated, with why (11.2) |
| `values.rs` | Generators: edge cases first, then seeded random values (11.3) |
| `shrink.rs` | Smaller values, to find the smallest failing input (11.4) |
| `found.rs` | The `tests/found/*.kavf` format: writing and reading calls (11.4) |
| `fixes.rs` | Fix suggestions for each kind of crash (11.10) |
| `keep.rs` | `kavach test --keep`: found cases that now pass, as `example` clauses (11.4) |

Only the program's own functions are tested, not the standard library modules every program has. Functions that can't be called with generated values are listed as not generated, with the reason (spec 11.2): generic ones, those taking values with no generator, those using a capability with no fake, and C functions and what reaches them.
