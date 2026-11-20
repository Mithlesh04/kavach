import { codeLines } from "../code-block";
import { ScenePlayer } from "./scene-player";
import { sceneCode, stepsText } from "./scene-text";
import { KIT_SCENES, sceneInfo } from "./scenes";
import type { SceneInfo } from "./types";

// What every explainer has around its animation, written by the server
// into the page's HTML: the "Caveman says" summary above it, and "Every
// step, as text" below it (closed until opened), so readers, search
// engines and AI tools get every step without running anything. The page
// puts its <SrcRef> links under the scene, where scripts/check-refs.mjs
// checks them.

function Caveman({ text }: { text: string }) {
  return (
    <aside aria-label="Caveman says" className="flex max-w-[72ch] flex-wrap items-baseline gap-x-3 gap-y-0.5 rounded-lg border border-line bg-accent-soft px-4 py-2.5">
      <span className="shrink-0 pt-0.5 text-[11px] font-extrabold uppercase tracking-wider text-accent-text">Caveman says</span>
      <span className="text-[15px] font-semibold leading-snug text-ink">{text}</span>
    </aside>
  );
}

function StepsText({ info, lines }: { info: SceneInfo; lines: string[] }) {
  return (
    <details className="rounded-lg border border-line px-4 py-2.5">
      <summary className="cursor-pointer text-[14px] font-bold">Every step, as text</summary>
      <ol className="m-0 mt-2 flex list-decimal flex-col gap-2 pl-5 text-[14px] leading-relaxed">
        {stepsText(info, lines).map((s) => (
          <li key={s.n}>
            <span className="text-soft">Line {s.line}:</span> <code className="break-all font-mono text-[13px] text-ink">{s.code}</code>
            <br />
            <span className="text-muted">{s.what}</span>
          </li>
        ))}
      </ol>
    </details>
  );
}

/** The frame around any animation: the caveman line, the animation, every step as text. */
export function SceneShell({ info, lines, children }: { info: SceneInfo; lines: string[]; children: React.ReactNode }) {
  return (
    <section id={`scene-${info.id}`} aria-label={info.title} className="flex min-w-0 scroll-mt-20 flex-col gap-3">
      <Caveman text={info.caveman} />
      {children}
      <StepsText info={info} lines={lines} />
    </section>
  );
}

/**
 * A scene the kit draws (components/explain/scenes.ts), with its checked
 * sample's code. The last thing the scene prints must be what the sample
 * really printed.
 */
export async function Explainer({ id }: { id: string }) {
  const info = sceneInfo(id);
  const scene = KIT_SCENES[id];
  if (!scene) throw new Error(`Explainer: "${id}" is not drawn by the kit; use its own component`);
  const { code, lines, output } = sceneCode(info);
  const printed = [...scene.frames].reverse().find((f) => f.output !== undefined)?.output;
  if (printed !== undefined && printed !== output) {
    throw new Error(`scene "${id}": its last step prints ${JSON.stringify(printed)}, but ${info.sample} prints ${JSON.stringify(output)}`);
  }
  return (
    <SceneShell info={info} lines={lines}>
      <ScenePlayer scene={scene} html={await codeLines(code)} />
    </SceneShell>
  );
}
