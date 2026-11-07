import fs from "node:fs";
import path from "node:path";
import Link from "next/link";
import { codeHtml } from "@/components/code-block";
import { Logo } from "@/components/logo";
import { SiteFooter } from "@/components/site-footer";
import project from "@/data/project.json";
import { ratioOf, winsAgainstCpp } from "@/components/ratio";
import { ThemeToggle } from "@/components/theme-toggle";
import versions from "@/data/versions.json";

// The benchmark itself, read from the repository: what runs is what shows.
const hero = fs.readFileSync(path.join(process.cwd(), "..", "benchmarks", "loops", "kavach", "loop.kav"), "utf8").trimEnd();

export default async function Home() {
  const heroHtml = await codeHtml(hero);
  const loop = ratioOf("loops", "loop");
  const sort = ratioOf("loops", "sort");
  const dict = ratioOf("lookups", "dict", "kavach", "python");
  const record = winsAgainstCpp();
  const ms = (v: number) => v.toLocaleString("en-US");
  return (
    <div className="flex min-h-screen flex-col">
      <header className="border-b border-line">
        <div className="mx-auto flex min-h-16 w-full max-w-[1200px] min-[1600px]:max-w-[1480px] items-center gap-6 px-8 max-md:gap-4 max-md:px-4 max-sm:gap-3 max-sm:px-3 max-[359px]:gap-2">
          <Link href="/" className="flex shrink-0 items-center gap-2.5 text-[19px] font-extrabold tracking-tight text-ink no-underline max-sm:gap-1.5">
            <Logo size={26} />
            Kavach
            <span lang="hi" className="text-[0.8em] font-semibold text-soft max-sm:hidden">
              {project.nameHindi}
            </span>
          </Link>
          <nav aria-label="Main" className="flex gap-5 text-[14.5px] font-semibold max-sm:gap-3 max-sm:text-sm max-[359px]:gap-2 max-[359px]:text-[13px]">
            <Link href="/docs/start/kavach-home/" className="text-muted no-underline">Learn</Link>
            <Link href="/docs/benchmarks/" className="text-muted no-underline max-[359px]:hidden">Benchmarks</Link>
            <a href={versions.repo} className="text-muted no-underline max-sm:hidden">GitHub</a>
          </nav>
          <div className="flex-1" />
          <ThemeToggle />
        </div>
      </header>
      <main id="main">

      <section className="bg-[#171412] text-[#f3efe7]">
        <div className="mx-auto flex w-full max-w-[1200px] min-[1600px]:max-w-[1480px] flex-wrap items-center gap-14 px-8 pb-16 pt-[72px] max-md:gap-10 max-md:px-4 max-md:pb-12 max-md:pt-12 min-[1600px]:gap-20">
          <div className="min-w-0 flex-[1_1_440px]">
            <p className="mb-3.5 text-[13px] font-bold uppercase tracking-[0.1em] text-[#e6a23c]">
              A compiled, statically typed programming language
            </p>
            <h1 className="mb-[18px] text-[clamp(2.25rem,0.5rem+3.4vw,4rem)] font-extrabold leading-[1.08] tracking-tight">
              Safety you can read.
              <br />
              Speed you can verify.
            </h1>
            <p className="mb-7 max-w-[56ch] text-lg leading-relaxed text-[#c9c2b6] max-sm:text-[17px] min-[1600px]:text-xl">
              Every integer checked, every index checked, no garbage collector, and still faster
              than C++ in {record.wins} of our {record.total} published benchmarks, with the losses
              published too. Every claim links to the line of code and the measurement behind it.
            </p>
            <div className="flex flex-wrap gap-3.5">
              <Link
                href="/docs/start/kavach-home/"
                className="rounded-[10px] bg-[#b45309] px-6 py-[13px] text-[15.5px] font-bold text-[#fff8ee] no-underline"
              >
                Start learning
              </Link>
              <Link
                href="/docs/benchmarks/"
                className="rounded-[10px] border border-[#4a443c] px-6 py-[13px] text-[15.5px] font-bold text-[#f3efe7] no-underline"
              >
                See the numbers
              </Link>
            </div>
          </div>
          <figure className="m-0 min-w-0 flex-[1_1_420px]">
            <div className="overflow-hidden rounded-xl border border-[#2b2721] bg-[#100e0c]">
              <div className="flex items-center gap-2 border-b border-[#2b2721] px-3.5 py-2.5">
                <span className="h-2.5 w-2.5 rounded-full bg-[#3a352e]" />
                <span className="h-2.5 w-2.5 rounded-full bg-[#3a352e]" />
                <span className="flex-1" />
                <span className="font-mono text-xs text-[#857c6e]">loop.kav</span>
              </div>
              <div
                className="code-body px-5 py-[18px] font-mono text-[13.5px] leading-[1.75] max-sm:px-4 max-sm:text-[12.5px] min-[1600px]:text-[15px] [&>pre]:m-0 [&>pre]:w-0 [&>pre]:min-w-full [&>pre]:overflow-x-auto [&>pre]:bg-transparent!"
                dangerouslySetInnerHTML={{ __html: heroHtml }}
              />
              <div className="flex flex-wrap gap-[18px] border-t border-[#2b2721] px-5 py-3 font-mono text-[12.5px]">
                <span className="text-[#9bb97f]">kavach {ms(loop.ours.value)} ms</span>
                <span className="text-[#857c6e]">g++ -O2 {ms(loop.theirs.value)} ms</span>
                <span className="text-[#e6a23c]">
                  {loop.shown}× faster, on every core, and it panics exactly where the plain loop would
                </span>
              </div>
            </div>
          </figure>
        </div>
      </section>

      <section className="border-b border-line bg-panel">
        <div className="mx-auto grid w-full max-w-[1200px] min-[1600px]:max-w-[1480px] grid-cols-3 gap-7 px-8 py-9 max-md:grid-cols-1 max-md:gap-5 max-md:px-4">
          {[
            [`${sort.shown}×`, "vs C++", "sorting 5 million integers: the buckets sort on every core"],
            [`${dict.shown}×`, "vs Python", "random reads from a 2-million-entry map: one cache miss per lookup"],
            ["100%", "checked", "every integer operation and list index is checked at these speeds"],
          ].map(([big, small, text]) => (
            <div key={big}>
              <p className="m-0 text-[40px] font-extrabold tracking-tight text-ink">
                {big} <span className="text-base font-semibold text-soft">{small}</span>
              </p>
              <p className="m-0 mt-1.5 text-[14.5px] leading-normal text-muted">{text}</p>
            </div>
          ))}
        </div>
      </section>

      <section className="mx-auto w-full max-w-[1200px] min-[1600px]:max-w-[1480px] px-8 py-14 max-md:px-4 max-md:py-10">
        <h2 className="m-0 mb-5 text-[28px] font-extrabold tracking-tight">What kind of language is Kavach?</h2>
        <div className="mb-8 grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5 max-lg:[&>:last-child]:sm:col-span-2">
          {[
            ["Compiled", "kavach build turns a program into one native executable. No interpreter or virtual machine runs it."],
            ["Quick to try, like a script", "A file can be a single print line, and kavach run starts it at once while you learn."],
            ["Statically typed", "Types are checked before anything runs, and worked out for you: you rarely write them."],
            ["No garbage collector", "Memory is freed the moment nothing uses it, so there are no pauses."],
            ["Built with Rust", "The compiler and runtime are written in Rust and compile through LLVM. Much of the standard library is written in Kavach itself."],
          ].map(([title, text]) => (
            <div key={title} className="rounded-xl border border-line bg-panel px-5 py-4">
              <p className="m-0 mb-1 text-base font-extrabold text-ink">{title}</p>
              <p className="m-0 text-[14.5px] leading-relaxed text-muted">{text}</p>
            </div>
          ))}
        </div>
        <p className="m-0 mb-6 max-w-[64ch] text-base text-muted">
          The course teaches one idea per page, from your first program to the GPU. Every example runs, and every
          claim links to its proof.
        </p>
        <div className="flex flex-wrap gap-3">
          <Link
            href="/docs/start/kavach-home/"
            className="inline-block rounded-[10px] bg-inv-bg px-6 py-[13px] text-[15.5px] font-bold text-inv-ink no-underline"
          >
            Start the course →
          </Link>
          <Link
            href="/docs/start/what-kavach-does-for-you/"
            className="inline-block rounded-[10px] border border-line px-6 py-[13px] text-[15.5px] font-bold text-ink no-underline"
          >
            What Kavach does for you
          </Link>
          <Link
            href="/docs/start/why-kavach/"
            className="inline-block rounded-[10px] border border-line px-6 py-[13px] text-[15.5px] font-bold text-ink no-underline"
          >
            Why another language?
          </Link>
        </div>
      </section>

      </main>
      <SiteFooter width="max-w-[1200px] min-[1600px]:max-w-[1480px]" />
    </div>
  );
}
