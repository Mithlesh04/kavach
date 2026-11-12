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

- **Types.** Numbers and booleans are passed the same way C passes them: `Int` and `UInt64` are 64-bit integers (`int64_t`, `uint64_t`), `Int32`, `Int16`, `Int8` and the other `UInt` types match their C sizes, `Float` is a `double` and `Bool` a C `bool`. `Ptr<T>` (6.4) is a C pointer `T*`, as a parameter or a result; a `Ptr` to a record is a pointer to a C `struct`. A record itself never goes to C by value. A `String` parameter is a `const char*`: C gets a copy of the text ending in a zero byte, valid only during the call; a text with a zero byte in it panics. A `Bytes` parameter is a `const uint8_t*`: C gets a copy of the bytes, valid only during the call, and is told how many by another argument, as C functions are. A function without `->` returns `void`. A C function can't give a `String`: it gives a `Ptr<UInt8>`, and `p.text()` copies the text (6.4); freeing the C memory, if the library says to, is up to the program. Other types are a compile error for now.
- **Callbacks.** A parameter of a function type, such as `fn(Ptr<Int32>, Ptr<Int32>) -> Int32`, is a C function pointer, with C types only. Its argument is the name of a Kavach function of exactly that type, which can't fail (no `!`): C calls it directly. A lambda or a closure can't be passed, because C has nowhere to keep what it captures. A panic inside the callback ends the program, as it does anywhere else.
- **Libraries.** The library is linked by name: `"z"` is `libz` (`-lz`) on Linux and macOS and `z.lib` on Windows. `"c"` (the C library) and `"m"` (its math part) are always there.
- **Native code only.** C functions run in native code (`kavach build`, `kavach run --native`). The interpreter stops at the first call of one and says to run the program natively.

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

### Python and JavaScript (Phase 3)

```kavach
extern python "numpy" as np
extern js "lodash" as _
```

Calls into Python or JS need the capability `Foreign`, because Kavach cannot check what foreign code does.
