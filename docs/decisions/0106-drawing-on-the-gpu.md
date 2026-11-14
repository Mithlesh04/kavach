# 0106. Drawing on the GPU: shaders as `gpu fn`s, a thin `std.gpu`

- **Status:** Accepted
- **Date:** 2026-11-14

## Context

Kavach computes on the GPU (`gpu fn`, decision 0044) and opens windows (`std.window`, decision 0104), but it can't draw. A maintainer's 3D game has an engine test from December 2026: a million particles drawn at 60 frames a second. User interfaces, charts and tools need drawing too.

Drawing needs shaders (small programs the GPU runs per corner and per pixel), pipelines that hold them, buffers of vertices, and a surface on a window. Every such API also leaves choices: what language shaders are written in, how much std provides, how data the GPU already holds is drawn, and when shaders compile. A maintainer chose all of them (2026-11-14).

## Decision

- **Shaders are `gpu fn`s:** `gpu vertex fn` and `gpu fragment fn` (spec 15.3), checked by the compiler as any `gpu fn` is, compiled through the same WGSL backend, and runnable on the CPU.
  - A vertex fn gives the corner's place in its result's field `position` (four `Float32`s). Its other fields go to the fragment fn.
  - A vertex fn takes the vertex, the instance and the value shared by the draw, in that order. The draw call names `instances:` and `with:`, and the compiler checks the types.
- **`std.gpu` is a thin typed layer** (spec 14.24): a screen on a window, frames, clearing, pipelines and draws; next, depth, textures and drawing into a texture. Renderers, scene graphs and game frameworks are packages.
- **No copy for GPU data:** a list a GPU `map` made and left in GPU memory (decisions 0055 and 0061) is drawn from there, as vertices or instances. A swarm moved by a `map` each frame never leaves the GPU.
- **Pipelines are built ahead:** `kavach build` compiles each pipeline and keeps it with the program, so no shader compiles mid-game. `kavach run` compiles at the first draw.
- **`Window` is the capability:** drawing changes the program's own window.
- **Vsync is on:** `frame()` waits for the screen's refresh, so a frame loop runs at the screen's rate.

## Consequences

- Shaders get Kavach's types, its checks, and CPU runs for debugging, with no second language.
- `runtime/gpu` keeps the instance it opened the device with, so surfaces come from the same GPU. `runtime/window` shares its windows with it.
- Built in stages: a screen and clearing first, then pipelines and draws from lists, then instances, shared values and GPU lists without copies, then depth, textures and the build-time cache.
- Vector types (`Vec4` and the others, language request 9 (5)) will replace the four-`Float32` tuples.

## Alternatives considered

- **A `shader { }` block, or WGSL text:** quicker to build, but shaders would go unchecked by Kavach's types and couldn't run on the CPU.
- **A renderer in std:** more out of the box, but std would carry one engine's choices; packages can compete instead.
- **Explicit GPU buffers:** more control, but a GPU `map`'s results would need copying into them every frame.
- **Compiling pipelines at first use only:** simpler, but a game would stall the first time each pipeline draws.
