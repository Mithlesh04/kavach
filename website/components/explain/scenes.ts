import { SCENARIOS } from "../memory-scenarios";
import { callScene } from "./scene-call";
import { chainScene } from "./scene-chain";
import { channelScene } from "./scene-channel";
import { compilerScene } from "./scene-compiler";
import { coresLoopScene } from "./scene-cores-loop";
import { dictLookupScene } from "./scene-dict-lookup";
import { factorialScene } from "./scene-factorial";
import { fileIoScene } from "./scene-file-io";
import { gpuDrawScene } from "./scene-gpu-draw";
import { gpuMapScene } from "./scene-gpu-map";
import { listBufferScene } from "./scene-list-buffer";
import { printScene } from "./scene-print";
import { httpsScene } from "./scene-https";
import { sortingScene } from "./scene-sorting";
import { spawnScene } from "./scene-spawn";
import { spawnKindsScene } from "./scene-spawn-kinds";
import { tickerScene } from "./scene-ticker";
import { turnsScene } from "./scene-turns";
import type { Scene, SceneInfo } from "./types";

// Every explainer scene on the site, by id: the ones the kit draws
// (KIT_SCENES), and the memory step-throughs, which keep their own
// picture (components/memory-blocks.tsx) and data (memory-scenarios.ts).
// A page shows a scene with <Explainer id="…" /> or <MemorySteps name="…" />;
// scene-pages.ts finds them there, for /explainers.txt and the page's data.

/** The scenes the kit draws, by id. */
export const KIT_SCENES: Record<string, Scene> = {
  [factorialScene.id]: factorialScene,
  [listBufferScene.id]: listBufferScene,
  [dictLookupScene.id]: dictLookupScene,
  [sortingScene.id]: sortingScene,
  [chainScene.id]: chainScene,
  [fileIoScene.id]: fileIoScene,
  [gpuMapScene.id]: gpuMapScene,
  [gpuDrawScene.id]: gpuDrawScene,
  [spawnScene.id]: spawnScene,
  [turnsScene.id]: turnsScene,
  [tickerScene.id]: tickerScene,
  [channelScene.id]: channelScene,
  [spawnKindsScene.id]: spawnKindsScene,
  [coresLoopScene.id]: coresLoopScene,
  [callScene.id]: callScene,
  [compilerScene.id]: compilerScene,
  [printScene.id]: printScene,
  [httpsScene.id]: httpsScene,
};

/** Each memory step-through's title and caveman summary, by scenario name. */
const MEMORY: Record<string, { title: string; caveman: string }> = {
  share: { title: "Does b = a take new memory?", caveman: "Two name. One list. No copy. Count go up. Compiler look ahead. Know last use. Let go there." },
  move: { title: "A move: the last use hands the object over", caveman: "Compiler look ahead. Old name not used again? Hand list over. No copy. Count stay same." },
  numbers: { title: "Numbers are copied, 8 bytes at a time", caveman: "Number small. Copy number. No need share. Change one? Other same." },
  copy: { title: "A list changed while shared is copied then", caveman: "Two name. One list. Change list? Copy first. Now two list." },
  text: { title: "Text is shared and copied as a list is", caveman: "Text same as list. Share. Change? Copy first." },
  loop: { title: "A list with one holder changes in place", caveman: "One name hold list. Push. No copy. List grow where it is." },
  dict: { title: "A dict keeps its entries in order, and an index past 8", caveman: "Dict keep order. Small dict: no index. Big dict: index. Find fast." },
  "dict-copy": { title: "A dict changed while shared is copied then", caveman: "Two name. One dict. Change dict? Copy first." },
  set: { title: "A set is a dict with keys only", caveman: "Set is dict, keys only. Same key again? Nothing happen." },
  fused: { title: "A fused chain reads the list as it was", caveman: "Map then filter. One loop, no list between. Change list after? Answer stay same." },
  record: { title: "A record that never becomes an object", caveman: "Record of numbers. Only used here. No box. Each field own spot. Copy field. Done." },
  freeze: { title: "Freezing costs nothing", caveman: "Freeze list. Same list. No copy. Compiler say: no change now." },
  shared: { title: "Shared: two names, one changing value", caveman: "Shared is one box. Two name hold box. One change, both see." },
};

/** The scene id of memory scenario `name`. */
export const memoryId = (name: string) => `memory-${name}`;

function memoryInfo(name: string): SceneInfo {
  const meta = MEMORY[name];
  if (!meta) throw new Error(`memory scenario ${name}: give it a title and a caveman line in components/explain/scenes.ts`);
  return { id: memoryId(name), ...meta, sample: `memory/step-${name}`, code: "main", steps: SCENARIOS[name] };
}

/** Every scene, by id, with what the text versions need. */
export const SCENES: Record<string, SceneInfo> = Object.fromEntries([
  ...Object.keys(SCENARIOS).map((name) => [memoryId(name), memoryInfo(name)] as const),
  ...Object.values(KIT_SCENES).map((s) => [s.id, { ...s, steps: s.frames }] as const),
]);

/** A scene's info, or a build error naming the file to fix. */
export function sceneInfo(id: string): SceneInfo {
  const info = SCENES[id];
  if (!info) throw new Error(`no explainer scene "${id}" (components/explain/scenes.ts)`);
  return info;
}
