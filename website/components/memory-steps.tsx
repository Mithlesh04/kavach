import { sampleBody } from "@/lib/samples";
import { MemoryBlocks } from "./memory-blocks";
import { SCENARIOS } from "./memory-scenarios";

/**
 * The step-through for `memory/step-<name>.kav`. Its code and output come
 * from the checked sample, and each frame must name a line that is there,
 * so a changed sample fails the build instead of showing a wrong picture.
 */
export function MemorySteps({ name }: { name: string }) {
  const frames = SCENARIOS[name];
  if (!frames) throw new Error(`MemorySteps: no scenario ${name} (components/memory-scenarios.ts)`);
  const sample = sampleBody(`memory/step-${name}`);
  const lines = sample.code.split("\n");
  for (const f of frames) {
    if (!lines[f.line]?.includes(f.at)) {
      throw new Error(`memory/step-${name}: a frame expects "${f.at}" on line ${f.line}, which is "${lines[f.line] ?? ""}" (components/memory-scenarios.ts)`);
    }
  }
  return <MemoryBlocks name={name} lines={lines} output={sample.output ?? ""} />;
}
