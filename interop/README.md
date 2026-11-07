# interop/

Lets Kavach use existing code, so nobody has to rewrite their stack to adopt it.

| Folder | Lets you | Phase |
|---|---|---|
| `c/` | Call C libraries (and anything with a C ABI) | 2 |
| `python/` | Use Python packages such as NumPy or PyTorch | 3 |
| `js/` | Use npm packages; run Kavach inside Node.js | 3 |

See [spec chapter 10](../docs/spec/10-modules-and-packages.md#107-interop) for the `extern` syntax.

**Status:** C works in native code for functions that take numbers, booleans, `Ptr`s, text and callbacks and give numbers, booleans and `Ptr`s: `extern c "name" { fn … }` is parsed into ordinary functions marked with their library, the checker keeps calls inside `unsafe`, and the code generators call the C symbol (`compiler/kir/src/lower_c.rs`); text goes as a zero-terminated copy and comes back through `p.text()`, and `alloc`, `free`, `read` and `write` work on C's memory (`compiler/kir/src/lower_ptr.rs`). The folders here hold the parts that need more than the compiler: none yet. A callback is a named Kavach function that can't fail, passed as its bare address. The other way round, `kavach build --lib` makes a Kavach library into a shared library with a C header (spec 10.10). Records reach C through `Ptr`s, laid out as C structs. Not yet: structs passed by value, and C functions with `...` (variadic).
