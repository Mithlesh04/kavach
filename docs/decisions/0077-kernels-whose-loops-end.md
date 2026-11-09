# 0077. Kernels whose loops always end skip wgpu's loop counter

- **Status:** Accepted
- **Date:** 2026-11-08 (Mithlesh chose this option)

## Context

`wgpu` compiles each kernel with a hidden counter in every loop
(`force_loop_bounding`), so a kernel whose loop never ends stops after a
very large number of rounds instead of running until the driver resets
the GPU. The counter costs a register, an add and a test on every round.
Where the kernel is the limit, it costs 8–18%: the 800-step stream of
[benchmarks/gpu](../../benchmarks/gpu/README.md) took 5.3 s with it and
4.3 s without it, against CUDA's 1.7 s.

Turning it off is `wgpu`'s `create_shader_module_trusted`, an `unsafe`
function: the caller promises every loop ends. `unsafe` is allowed only in
`runtime/native` ([decision 0016](0016-native-runtime-and-executables.md),
AGENTS.md section 8), and the call belongs in `runtime/gpu`, which owns the
device.

## Decision

**The compiler marks a kernel whose every loop surely ends, and only
such a kernel is compiled without the counter.**

1. A loop surely ends when it is a `for` over an exclusive range (`a..b`):
   the end is worked out once, the variable goes up by one each round, and
   nothing else changes it, so it reaches the end before it could wrap.
   A `while`, and an inclusive range (`a..=b`, which wraps at the largest
   number and never ends), keep the counter. WGSL has no recursion.
2. `compiler/backend/wgsl` writes `// loops end` among a kernel's first
   lines when every loop it wrote is such a `for`.
3. `runtime/gpu` compiles a kernel with that line through
   `create_shader_module_trusted`, with the counter off and every other
   check on (bounds checks stay). This is the only `unsafe` in
   `runtime/gpu`, with a `SAFETY:` comment; the line comes only from the
   compiler, as a program can't write kernel text.

## Consequences

- Kernels of `for` loops run up to 18% faster where the kernel is the
  limit; everything else is unchanged.
- A second crate has `unsafe`: one call, whose safety rests on the
  compiler's mark. A change to how the WGSL backend writes loops must keep
  the rule in point 1, and its tests check that a `while` and an inclusive
  range get no mark.
- AGENTS.md section 8 names this exception.

## Alternatives considered

- **Keep the counter everywhere:** no new `unsafe`, kernels stay slower.
- **Turn it off for every kernel:** fastest, but a `gpu fn` whose `while`
  never ends could hang the GPU until the driver resets it.
