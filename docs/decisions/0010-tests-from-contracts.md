# 0010. Tests are generated from types and contracts

- **Status:** Accepted
- **Date:** 2026-10-30

## Context

In most languages, testing means writing one test per scenario: empty input, negative number, service down. This is slow, so it gets skipped, and the scenarios nobody thought of are exactly the ones that break in production.

Kavach code already describes itself unusually well. Types say what values are possible. Contracts (`require`, `ensure`, `invariant`) say what is valid and what is promised. Capabilities say every way code touches the outside world, and `! E` lists every error it can return. That is almost everything a test needs.

## Decision

The goal is **close to zero hand-written test files**:

- `kavach test` generates inputs for every function from its parameter types, guided by `require` and `invariant`, and checks `ensure`, invariants and the absence of panics.
- A new one-line `example` clause in signatures gives concrete cases that double as documentation.
- Capabilities are faked automatically, returning successes and every declared error, so error paths are tested without extra code.
- Agents are explored: generated events, every possible `ask<T>` answer, approvals approved/rejected/timed out, and limits pushed to their edges.
- Failing inputs are shrunk, saved in `tests/found/` and replayed forever.
- Every crash comes with suggested fixes (return an error, add a `require`, or use a default), which the developer chooses.
- Hand-written `test` blocks stay, for multi-step stories.

(Spec chapter 11, section 7.6.)

## Consequences

- Writing contracts pays twice: clearer code and free tests. This encourages contracts, which also help reviewers and AI.
- Error handling and agent guardrails are tested by default, where bugs are most expensive.
- `kavach test` is more work to build than a plain test runner: generators, shrinking, fakes and exploration. It is phased across Phases 1–3.
- Generated testing is only as strong as the contracts. The test report lists weakly tested functions honestly instead of showing a coverage percentage.

## Alternatives considered

- **Plain unit tests only (most languages):** proven, but they test only the scenarios someone thought of, and depend on discipline.
- **Property testing as a library (QuickCheck, Hypothesis, proptest):** powerful, but opt-in, needs generators written by hand, and can't fake side effects without a capability system.
- **AI writes the test files:** still leaves many test files to read and maintain, and tests can encode wrong behaviour. Kavach uses AI only to *suggest* contracts and examples, which people review.
- **Proof instead of tests:** the long-term direction for critical code (Phase 4, `kavach verify`), but too hard for everyday use.
