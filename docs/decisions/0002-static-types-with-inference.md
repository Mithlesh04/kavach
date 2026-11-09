# 0002. Static types with inference

- **Status:** Accepted, amended by [0012](0012-return-types-are-written.md) (return types)
- **Date:** 2026-10-30

## Context

Kavach must be fast (types enable native code), safe (capabilities, secrets and contracts rely on types), and easy (types should not get in the way).

## Decision

Fully static types. Local types are always inferred. Types are required only on function parameters, record fields and `pub` return types (spec 2.10).

## Consequences

- Most code reads like a dynamic language while keeping full compile-time checking.
- Public signatures are always explicit, which makes them a reliable contract for people and AI to review.
- No gradual or dynamic mode: one set of rules everywhere. Scripts get ease through inference and top-level statements instead.

## Alternatives considered

- **Gradual typing (TypeScript or Python style):** easy to start with, but the guarantees that make Kavach worth adopting (capabilities, secrets) would have holes wherever types are missing.
- **Types required everywhere:** maximum explicitness, but too much ceremony.
