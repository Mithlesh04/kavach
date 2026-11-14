# 0109. Operators for a program's own types, through interfaces

- **Status:** Accepted
- **Date:** 2026-11-15

## Context

Kavach had no operator overloading, so a number type written in Kavach could only have methods (`a.add(b)`); decision 0041 made `BigInt` built in for that reason and left "operator interfaces" for later. Sakshi, a chip-design tool written in Kavach, asked for them (language request 10 (4)): fixed-point numbers, complex numbers, intervals, units and vectors should be packages that read like numbers. A maintainer chose interfaces (2026-11-14), and that the other operand may be the same type or one other type per operator (2026-11-15).

## Decision

- Built-in interfaces `Add`, `Sub`, `Mul`, `Div`, `Rem` and `Neg` give `+ - * / %` and unary `-`; `Comparable` (`fn compare(self, other: Self) -> Int`) gives `< <= > >=` (spec 2.16).
- `Add` takes the same type and gives it; `Add<R>` takes an `R` and gives `Self`; `Add<R, Out>` gives an `Out`. A type implements each once: its method has one name, and Kavach has no overloading (spec 1.4).
- Your type is on the left: `a * 2.0` calls `a.mul(2.0)`; `2.0 * a` is a compile error that says to swap them.
- Operator methods are pure (no `uses`) and can't fail (no `!`), so an operator never hides I/O or an error.
- After checking, the compiler turns each such operator into the method call it stands for (`a + b` into `a.add(b)`, `a < b` into `a.compare(b) < 0`), so the interpreter and native code need nothing new.
- `==` stays structural for every type (1.8), not an interface.
- Built-in types keep their own operators.

## Consequences

- Number-like types move out of the compiler into packages; `BigInt` and `Decimal` stay built in (0041).
- `x op= v` works for them, as `x = x op v`.
- Generic functions that ask for an operator (`T: Add`), `sort()`, `min()` and `max()` of `Comparable` types, and bit operators, are open questions (spec 2.16).

## Alternatives considered

- **Same type only:** simplest, but no `v * 2.0` and no units whose product is another type.
- **Any number of operand types** (several `Mul<…>` per type, methods named by the compiler): overloading in another form, which spec 1.4 rules out.
- **Operators as free functions** (C++'s `operator+`): found by searching every module; an interface is declared on the type, where `kavach query type` and the outline show it.
- **`2.0 * v` through a reversed interface** (Python's `__rmul__`): a second method for the same operator, and a rule for which wins; left out for one clear rule.
