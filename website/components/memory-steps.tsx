import { codeLines } from "./code-block";
import { SceneShell } from "./explain/explainer";
import { sceneCode } from "./explain/scene-text";
import { memoryId, sceneInfo } from "./explain/scenes";
import { MemoryBlocks } from "./memory-blocks";
import { checkPlaces } from "./memory-places";
import { SCENARIOS } from "./memory-scenarios";

/**
 * The step-through for `memory/step-<name>.kav`, in the explainer kit's
 * frame: the caveman line, the animation and every step as text. Its code
 * and output come from the checked sample, and each frame must name a line
 * that is there (explain/scene-text.ts), so a changed sample fails the
 * build instead of showing a wrong picture. checkPlaces fails the build
 * when the scenario would draw something moving (memory-places.ts).
 */
export async function MemorySteps({ name }: { name: string }) {
  const frames = SCENARIOS[name];
  if (!frames) throw new Error(`MemorySteps: no scenario ${name} (components/memory-scenarios.ts)`);
  checkPlaces(name, frames);
  const info = sceneInfo(memoryId(name));
  const { code, lines, output } = sceneCode(info);
  return (
    <SceneShell info={info} lines={lines}>
      <MemoryBlocks frames={frames} html={await codeLines(code)} output={output} />
    </SceneShell>
  );
}
