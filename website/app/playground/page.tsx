import fs from "node:fs";
import path from "node:path";
import type { Metadata } from "next";
import Link from "next/link";
import { Logo } from "@/components/logo";
import { SiteFooter } from "@/components/site-footer";
import { ThemeToggle } from "@/components/theme-toggle";
import { PlaygroundEditor, type Starter } from "@/components/playground-editor";
import project from "@/data/project.json";
import versions from "@/data/versions.json";

export const metadata: Metadata = {
  title: "Playground",
  description:
    "Write a Kavach program and the real compiler checks it, right in your browser.",
};

// Starters are checked samples (content/samples/), never strings here.
const sample = (...parts: string[]) =>
  fs
    .readFileSync(path.join(process.cwd(), "content", "samples", ...parts), "utf8")
    .trimEnd();

const starters: Starter[] = [
  { name: "An agent, caught", code: sample("agents", "agent-console.fails.kav") },
  { name: "Contracts", code: sample("contracts", "ensure.kav") },
  { name: "No silent coercion", code: sample("classic", "no-coercion.fails.kav") },
];

export default function Playground() {
  return (
    <div className="flex min-h-screen flex-col">
      <header className="border-b border-line">
        <div className="mx-auto flex min-h-16 w-full max-w-[920px] items-center gap-6 px-8 max-md:gap-4 max-md:px-4">
          <Link
            href="/"
            className="flex shrink-0 items-center gap-2.5 text-[19px] font-extrabold tracking-tight text-ink no-underline"
          >
            <Logo size={26} />
            Kavach
            <span lang="hi" className="text-[0.8em] font-semibold text-soft max-sm:hidden">
              {project.nameHindi}
            </span>
          </Link>
          <nav
            aria-label="Main"
            className="flex gap-5 text-[14.5px] font-semibold max-sm:gap-3 max-sm:text-sm"
          >
            <Link href="/docs/start/kavach-home/" className="text-muted no-underline">
              Learn
            </Link>
            <a href={versions.repo} className="text-muted no-underline max-sm:hidden">
              GitHub
            </a>
          </nav>
          <div className="flex-1" />
          <ThemeToggle />
        </div>
      </header>
      <main id="main" className="flex-1">
        <div className="mx-auto w-full max-w-[920px] px-8 pb-16 pt-10 max-md:px-4">
          <h1 className="mb-2 text-3xl font-extrabold tracking-tight">Playground</h1>
          <p className="mb-6 max-w-[64ch] text-[15px] leading-relaxed text-muted">
            The real compiler, built for WebAssembly, checks your program as you type —
            right here in your browser, nothing is sent anywhere. It checks; running
            programs still needs <code>kavach run</code> on your machine.
          </p>
          <PlaygroundEditor starters={starters} />
        </div>
      </main>
      <SiteFooter width="max-w-[920px]" />
    </div>
  );
}
