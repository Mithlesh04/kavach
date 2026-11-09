# 0008. Capabilities are part of function signatures

- **Status:** Accepted
- **Date:** 2026-10-30

## Context

Kavach's reason to exist is trust: knowing what code, especially AI-written code and AI agents, can do. A library cannot guarantee this, because any function anywhere could open a socket or call a payment API. Only the compiler can.

## Decision

- Functions declare what they touch with `uses`. Without `uses`, a function is pure.
- Outside systems are declared as `capability` types with swappable implementations.
- Agents declare grants with `can`, plus `limit` and `approve` rules enforced by the runtime.
- `pub` functions must write `uses`; private functions may have it inferred.

(Spec chapter 8.)

## Consequences

- A signature shows exactly what code can affect. Reviewers and auditors read signatures, not bodies.
- Testing is easy: swap a capability for a fake implementation.
- Some friction: adding a network call deep in a call chain changes the public signatures above it. This is deliberate. It is exactly the change a reviewer should see.

## Alternatives considered

- **Runtime-only permission checks:** easy to bypass and only found in production.
- **Full algebraic effect handlers (Koka, Unison):** more powerful, but harder to learn. Kavach keeps the useful part (tracking) without the complexity.
