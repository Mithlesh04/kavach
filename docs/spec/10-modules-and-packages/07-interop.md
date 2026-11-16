# 10.7 Interop

### C (Phase 2)

```kavach
extern c "m" {
    fn cbrt(x: Float) -> Float
}

fn cube_root(x: Float) -> Float {
    unsafe { cbrt(x) }
}
```

`extern c "name"` declares functions of a C library, by their C names and with their C types written as Kavach types. A call is `unsafe` (6.4): it goes inside `unsafe { … }`, so the package needs `unsafe = true`, and the usual way is a safe Kavach function around it, as `cube_root` is here.

- **Types.** Numbers and booleans are passed the same way C passes them: `Int` and `UInt64` are 64-bit integers (`int64_t`, `uint64_t`), `Int32`, `Int16`, `Int8` and the other `UInt` types match their C sizes, `Float` is a `double` and `Bool` a C `bool`. `Ptr<T>` (6.4) is a C pointer `T*`, as a parameter or a result; a `Ptr` to a record is a pointer to a C `struct`. A record of those types (6.4) goes to C and comes back **by value**, as a C `struct` does: laid out as C lays it out, and passed as the system's C passes a `struct`, in registers, on the stack or through a hidden pointer. This works on x86-64 (Windows, Linux, macOS on Intel) for now; elsewhere it is a compile error. A `String` parameter is a `const char*`: C gets a copy of the text ending in a zero byte, valid only during the call; a text with a zero byte in it panics. A `Bytes` parameter is a `const uint8_t*`: C gets a copy of the bytes, valid only during the call, and is told how many by another argument, as C functions are. A function without `->` returns `void`. A C function can't give a `String`: it gives a `Ptr<UInt8>`, and `p.text()` copies the text (6.4); freeing the C memory, if the library says to, is up to the program. Other types are a compile error for now.
- **Callbacks.** A parameter of a function type, such as `fn(Ptr<Int32>, Ptr<Int32>) -> Int32`, is a C function pointer, with C types only. Its argument is the name of a Kavach function of exactly that type, which can't fail (no `!`): C calls it directly. A lambda or a closure can't be passed, because C has nowhere to keep what it captures. A panic inside the callback ends the program, as it does anywhere else.
- **More arguments.** A C function whose parameters end in `...`, as `printf`'s do, takes more arguments after its own. Each goes as its type is, after C's promotions: `Int8`, `Int16`, `UInt8` and `UInt16` become a C `int`, and a `Float32` a `double`. A number literal there is an `Int` (`int64_t`) or a `Float` (`double`), so write `Int32(42)` where C reads an `int` (`%d`). Only numbers, `Ptr`s and `String`s go after `...`. A Kavach function has no `...`: it takes more values in a list (3.1).
- **Libraries.** The library is linked by name: `"z"` is `libz` (`-lz`) on Linux and macOS and `z.lib` on Windows. `"c"` (the C library) and `"m"` (its math part) are always there.
- **Libraries loaded while running.** `extern c library Name { … }` declares the functions of a library the program loads itself, for plugins and for libraries that may be missing: it isn't linked. `Name` is a type: `Name.open(path)` loads the library at `path` (a file name alone is looked for as the system does) and finds each function of the block in it, giving a `Name`; or it fails with a `LibraryError`, whose text says which library couldn't be loaded, and why, or which function it lacks. Each function is then a method of that value: `lib.add(2, 3)`. Loading runs the library's own start-up code, so `open` and each call go inside `unsafe { … }`. A loaded library stays loaded until the program ends.
- **Native code only.** C functions run in native code (`kavach build`, `kavach run --native`). The interpreter stops at the first call of one and says to run the program natively. `kavach run --native` finds a C function in the libraries loaded while it runs. Windows' C library has some of `printf`'s family only in its headers, as `snprintf`: in memory they can't be called, and `kavach run --native` says so before running. `kavach build` links them (`sprintf`, `printf` and others are found in the system's `msvcrt.dll`).

```kavach
extern c "c" {
    fn llabs(n: Int) -> Int
}
```

```kavach
extern c "c" {
    fn strlen(s: String) -> UInt64
    fn memset(p: Ptr<UInt8>, value: Int32, n: UInt64) -> Ptr<UInt8>
}

fn c_length(s: String) -> UInt64 {
    unsafe { strlen(s) }
}

extern c "c" {
    fn getenv(name: String) -> Ptr<UInt8>
}

fn home() -> String? {
    unsafe { getenv("HOME").text() }      // `none` when C gives a null pointer
}

extern c "c" {
    fn qsort(items: Ptr<Int32>, count: UInt64, size: UInt64, compare: fn(Ptr<Int32>, Ptr<Int32>) -> Int32)
}

fn by_value(a: Ptr<Int32>, b: Ptr<Int32>) -> Int32 {
    unsafe { a.read(0) - b.read(0) }
}

fn sort(items: Ptr<Int32>, count: UInt64) {
    unsafe { qsort(items, count, 4, by_value) }     // C calls `by_value`
}
```

```kavach
extern c "c" {
    fn snprintf(out: Ptr<UInt8>, size: UInt64, format: String, ...) -> Int32
}

fn money(cents: Int) -> String {
    unsafe {
        const out: Ptr<UInt8> = alloc(32)
        const _ = snprintf(out, 32, "%lld.%02d", cents / 100, Int32(cents % 100))
        const text = out.text() ?? ""
        free(out)
        text                                          // money(1234) is "12.34"
    }
}
```

```kavach
type LongDiv {
    quot: Int
    rem: Int
}

extern c "c" {
    fn lldiv(n: Int, d: Int) -> LongDiv              // C's `lldiv_t lldiv(long long, long long)`
}

fn split(cents: Int) -> (Int, Int) {
    const d = unsafe { lldiv(cents, 100) }
    (d.quot, d.rem)                                   // split(1234) is (12, 34)
}
```

```kavach
extern c library Adder {
    fn add(a: Int32, b: Int32) -> Int32
}

fn main() ! LibraryError {
    const lib = unsafe { try Adder.open("plugins/libadder.so") }
    print(unsafe { lib.add(2, 3) })                   // 5
}
```

### Python and JavaScript (Phase 3)

```kavach
extern python "numpy" as np
extern js "lodash" as _
```

Calls into Python or JS need the capability `Foreign`, because Kavach cannot check what foreign code does.
