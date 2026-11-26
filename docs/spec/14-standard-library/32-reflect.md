# 14.32 std.reflect

What a type or a module offers, for tools, plugins and libraries that adapt to the code they are given. One word for it everywhere: a type's or a module's **members**. Everything here is known while compiling, so it costs nothing to work out at run time.

```kavach
use std.math
use std.reflect.{MemberOrigin}

type Order {
    pub id: Int
    pub total: Float
}

extend String {
    fn shout(self) -> String { self.upper() + "!" }
}

fn main() {
    for m in Order.members() {
        print(`{m.kind} {m.name}: {m.result}`)   // Field id: Int, Field total: Float
    }
    for m in String.members() {
        if m.origin != MemberOrigin.BuiltIn {
            print(`{m.name}: {m.origin}`)         // shout: Extended(module: "main")
        }
    }
    print("hi".has_member("shout"))               // true
    print(math.members().map(m => m.name))        // ["factorial", "gcd", …]
}
```

| Call | Gives |
|---|---|
| `T.members() -> [Member]` | Every field and method of type `T`: built in, declared in its body, or added by an `extend`, sorted by name. `T` is a type's name: `String`, `Int`, `Order`, and `List`, `Dict`, `Set` and `Ring` for collections |
| `m.members() -> [Member]` | What module `m` offers to the code that uses it: its `pub` functions, types and constants, sorted by name |
| `x.has_member(name) -> Bool` | Whether `x`'s type has a field or method `name`; `name` is a text literal. For an interface value (2.15), whether the interface has it |
| `reflect.type_of(x) -> String` | `x`'s whole type as code writes it, where `kind_of` gives only its kind: `[Int]`, `{String: Float}`, `String?`, `Order` (*not built yet*; a maintainer asked 2026-11-27, the name for a maintainer's review). Known while compiling, except for a value whose type is only known when the program runs: an interface value (2.15) or an inline union (2.6) gives its own type then, `type_of(shape)` giving `Circle` |
| `reflect.kind_of(x) -> Kind` | What kind of data `x` is, as a value of the enum `reflect.Kind`, for checks and `match` (*not built yet*; a maintainer asked 2026-11-27): one variant per built-in type, `String`, `Char`, `Bool`, `Int`, `Int8` to `Int64`, `UInt8` to `UInt64`, `Float`, `Float32`, `Decimal`, `BigInt`, `Bytes`, `Json`; one per collection, `List`, `Dict`, `Set`, `Ring`, `Tuple`, `Tensor`; and `Record`, `Union`, `Enum`, `Optional`, `Function`, `Interface`, `Shared`, `Task`, `Channel` for the rest. A wider whole number (`Int<bits: 128>`) is `Int`, and a range number its base type. It prints as its name: `print(reflect.kind_of([1]))` prints `List`. An interface value or an inline union gives the kind of what it holds |

The program needs no `use std.reflect` for these calls; it does to name the types below, as `MemberOrigin` above.

```kavach
pub type Member {
    pub name: String
    pub kind: MemberKind
    pub params: [Param]       // a method's or function's, without `self`
    pub result: String        // what it gives ("Nothing" for none), a field's or constant's type, or a type's name
    pub origin: MemberOrigin
}

pub type MemberKind = Method | Field | Function | Type | Constant

pub type Param {
    pub name: String
    pub type_name: String     // as written: `[Int]`, `String?`
}

pub type MemberOrigin = BuiltIn | Declared | Extended(module: String)
```

- Types are written as in code: `[Int]`, `String?`, `fn(Int) -> Bool`. A built-in method's parameters are named `a`, `b`, … in order, as the compiler knows only their types.
- A generic type's members show its parameters' names: `Box<T>`'s `get` gives `T`. For a collection, `T` is the item type, and `K`, `V` a dict's key and value.
- Members that aren't `pub` are listed only inside their own package (10.1), where they can be used.
- A module's members are those it declares `pub` (10.1). For a standard library module, those are the parts written in Kavach: `math.members()` lists `gcd` but not yet `sin`, which the compiler provides.

## Open questions

- The standard library functions the compiler provides (`math.sin`, `fs.read_text`) in `m.members()`, with their signatures.
- `has_member` on an interface value asking the value's own type, at run time, rather than the interface.
- A member's documentation comment and examples in `Member`, for tools.
