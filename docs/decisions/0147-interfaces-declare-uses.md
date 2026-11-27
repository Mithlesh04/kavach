# 0147. Interface methods declare what they use

- **Status:** Accepted
- **Date:** 2026-11-26

## Context

The type checker's soundness review ([checker-review.md](../architecture/checker-review.md), finding 1) found that a function with no `uses` could print or write files by calling a method through an interface value or a generic bound (`T: Describe`). The checker gave interface methods no capabilities, and never compared an implementation's `uses` with the interface's. The spec (2.8, 2.15, 8.1) said nothing about how interfaces and capabilities meet. This broke Kavach's main promise: a signature shows what code may touch.

## Decision

A maintainer chose (2026-11-26):

1. **An interface method lists its `uses`**, as a function does (spec 8.1). One without `uses` is pure.
2. **An implementation may use only what the interface's method lists.** Using more is a compile error naming the interface method.
3. **Calling the method through the interface**, or through a generic bound, needs exactly what the interface's method lists, known from the interface alone.
4. The capabilities a package grants by default ([decision 0148](0148-default-capabilities.md)) count as listed, as they do for functions.

## Consequences

- The hole is closed for interface values and generic bounds alike, and `kavach audit` sees every effect from signatures.
- An interface written for printing or file access says so. Existing interfaces with pure methods are unchanged; an implementation that printed must now drop it or the interface must list it.
- The checker must record interface methods' needs (`capabilities.rs`) and compare `uses` in `conform.rs`.

## Alternatives considered

- **Pay when boxing:** making an interface value pays for its type's methods' capabilities, as making a function value does. Rejected: the cost shows where the value is made, not where the call is, and generic bounds need a second rule.
- **Both, with generics charged at each instantiation:** most flexible, most work, and the signature no longer tells the whole story.

## Built, and points for a maintainer's review

Built on 2026-11-27 (spec 2.8). Where this record didn't settle a point, the simplest option that fits it was taken. Each is **for a maintainer's review**:

1. **What an implementation uses is what it is compared with**, written or inferred: a private method without `uses` that reads a file breaks the rule as one that writes `uses Fs.read` does.
2. **The error is on the implementing method**, and names the interface's method: add the capability to the interface's method, or stop using it there.
3. **A contract can't call an interface's method that lists a capability** (7.5), as it can't call a function that does. With default capabilities ([decision 0148](0148-default-capabilities.md)), see that record's points.
