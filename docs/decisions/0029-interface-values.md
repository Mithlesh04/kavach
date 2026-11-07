# 0029. Interface values: runtime polymorphism without inheritance

- **Status:** Accepted
- **Date:** 2026-11-01

## Context

Kavach has three kinds of polymorphism: generics with interface bounds (`fn show<T: Describe>(x: T)`, compiled once per type, C-fast), unions (`match` over a closed set of variants), and structural anonymous records (decision 0028). What's missing is the everyday fourth (Mithlesh, 2026-11-01): holding *different* types behind one interface at run time — a list of handlers, a sink parameter any logger fits — without declaring a union of everything in advance. Go built its ecosystem on this; Rust has `dyn`; class languages use inheritance for it. Kavach has no inheritance, on purpose.

## Decision

**An interface name is also a value type. A value of any type that implements the interface fits, and method calls on it dispatch at run time.**

1. `fn notify(sink: Describe)` takes a `User`, a `Rect`, anything declared `: Describe` (spec 2.8). `let all: [Describe] = [user, rect]` holds them together.
2. Calling `sink.describe()` finds the concrete type's method at run time, through a method table built at compile time. No reflection, no fallback: the checker proved the type implements the interface when the value went in.
3. Only the interface's methods exist on the value. There is no downcast in this decision; a program that needs the concrete type back models that as a union.
4. Generics stay the fast path: `fn show<T: Describe>` monomorphizes and inlines. An interface value is the flexible path and pays one indirection per call, like Go. The compiler may still devirtualize when it proves the concrete type.
5. In native code an interface value is a boxed pair: the value and its method table (a fat pointer). The interpreter dispatches by the value's type directly.

## Consequences

- Runtime polymorphism without inheritance: composition plus interfaces, the Go model, with Kavach's contracts and capabilities on top.
- A second representation for values of interface type (box + table); `moves.rs` and reference counting treat it like any other heap value.
- Printing an interface value prints the concrete value. Equality between interface values is an open question (spec 2.8) until a use case decides it.
- Implementation is phased like anonymous records: checker and interpreter first, then native method tables; until then native builds refuse it as unsupported.

## Alternatives considered

- **Inheritance:** brings the fragile-base-class problem and deep hierarchies; interfaces plus composition cover the same uses.
- **Auto-unions (the compiler collects every implementing type):** closed-world, breaks separate compilation and packages.
- **Structural interface values (duck typing à la Go's implicit satisfaction):** spec 2.8 already decided implementing is explicit; keeping the value form nominal keeps one rule.
