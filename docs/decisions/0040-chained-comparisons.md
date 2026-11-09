# 0040. Comparisons chain, as in Python

- **Status:** Accepted
- **Date:** 2026-11-03

## Context

Mithlesh asked for `b == a == c` to mean `b == a and a == c`, with any mix of comparisons: `a == b != c < 4` (2026-11-03). Until now the parser refused two comparisons in a row, because in C, Java or JavaScript `a < b < c` compares the boolean `a < b` with `c`: a classic bug.

Python reads `a < b < c` as mathematics does: `a < b and b < c`. Readers already know it from mathematics, and it can't be the C bug, since Kavach never compares a `Bool` with a number.

## Decision

**A row of comparisons (`== != < <= > >=`) means each neighbouring pair compared, joined by `and` (spec 1.8).**

1. `a op1 b op2 c` is `a op1 b and b op2 c`, for any operators of the comparison group and any length.
2. Each operand is worked out once, left to right. `a < f() < c` calls `f` once.
3. It stops at the first link that is false: later operands are not worked out, as with `and`.
4. Only neighbours are compared: `a != b != c` says nothing about `a` and `c`.
5. Parentheses still group: `(a == b) == c` compares a `Bool` with `c`.
6. Ranges still don't chain: `0..5..9` is an error.

## Consequences

- Range checks read as written: `0 <= i < len`.
- The AST has a node of its own for a chain, so every operand keeps its place in the source for errors and tools, and is evaluated once without hidden variables.

## Alternatives considered

- **Keep refusing chains:** safe, but Mithlesh asked for the shorter form, and Python shows it is safe when booleans and numbers don't mix.
- **Rewrite in the parser to `a op b and b op c`:** simple, but `b` would be worked out twice, which is wrong when it has side effects or is slow.
