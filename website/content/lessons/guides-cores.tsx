import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { guideProject } from "./guide-parts";
import { Ex, H2, Note, P } from "./parts";

// ---------- Guides › Use Every Core and the GPU ----------
//
// One task from start to finish: guides/longest-chain.project finds the
// number below 100,000 with the longest Collatz chain, with one `gpu fn`
// run on one core, on every core and on the GPU, and checks the three
// agree. Whole numbers give the same answer on a GPU and a CPU (spec
// 15.2's float note doesn't apply), so the checked output is the same
// with or without a GPU.

const chain = guideProject("longest-chain", ["src/main.kav", "kavach.toml"]);
const main = "src/main.kav";

export const guideCoresDoc: LessonDoc = {
  lead: "One task from start to finish: find the number below 100,000 whose Collatz chain is longest, running one function on one core, on every core with on cores, and on the GPU with a map, and check that all three agree.",
  simple: <>A computer has several cores, and often a GPU with thousands of small ones. Work that splits into many independent pieces can use them all. Kavach lets you say where a loop runs, and makes sure the answer doesn&apos;t change.</>,
  minutes: 14,
  before: ["speed/loops-on-all-cores", "speed/gpu"],
  body: (
    <>
      <P>
        The work: for each number, count the steps to reach 1, halving an even number and turning an odd one into{" "}
        <C>3n + 1</C>. Each number is a piece of its own, which is what spreads well over cores.
      </P>
      <H2>1. One function, for the CPU and the GPU</H2>
      <P>
        A <C>gpu fn</C> can run on the GPU, and on the CPU too. It works on whole numbers, which give the same answer on
        both: a GPU may round floats differently, by fusing a multiply with an add. Its <C>require</C> keeps out 0
        and negative numbers, whose chains never reach 1:
      </P>
      <CodeBlock code={chain.piece(main, "gpu fn steps")} file={main} />
      <H2>2. On one core</H2>
      <P>A plain loop runs on one core, one number after another:</P>
      <CodeBlock code={chain.piece(main, "fn on_one_core")} file={main} />
      <H2>3. On every core</H2>
      <P>
        <C>on cores</C> before the loop shares its rounds between the cores. Each round writes only its own place,{" "}
        <C>out[i]</C>, so the rounds can run in any order and the list comes out the same:
      </P>
      <CodeBlock code={chain.piece(main, "fn on_every_core")} file={main} />
      <P>
        The compiler holds every <C>on cores</C> loop to that rule. Writing <C>out[n - 1]</C> instead, where two rounds
        might reach the same place, is refused:
      </P>
      <Ex name="guides/cores-own-slot.fails" />
      <H2>4. On the GPU</H2>
      <P>
        A <C>map</C> whose function is a <C>gpu fn</C> runs on the GPU when the list is big enough for the GPU to
        win and the machine has one. Otherwise the same <C>map</C> runs on the CPU, with the same list as a result:
      </P>
      <CodeBlock code={chain.piece(main, "fn on_the_gpu")} file={main} />
      <H2>5. Run it, and check the three agree</H2>
      <CodeBlock code={chain.piece(main, "fn main")} file={main} />
      <Terminal session={chain.said("check")} />
      <Terminal session={chain.said("run --native src/main.kav")} />
      <P>
        <C>--native</C> compiles to machine code first, as this is a lot of work for the interpreter. Built with{" "}
        <C>kavach build</C>, the <C>on cores</C> loop runs on all the program&apos;s cores at once. What each way costs
        on real machines is in <Link href="/docs/benchmarks/">Benchmarks</Link>.
      </P>
      <H2>6. Test it</H2>
      <P>
        Each function takes its numbers, so the tests try small lists and stay quick. Every one checks the same
        thing: the three ways give the same answer, and the generated tests try hundreds more lists:
      </P>
      <Terminal session={chain.said("test --seed 7")} />
      <Note>
        Fewer than 200 cases for some functions is normal: generated lists that break the <C>require</C> are left
        out. See <Link href="/docs/errors/contracts/how-they-are-checked/">How they are checked</Link>.
      </Note>
      <H2>The whole program</H2>
      <CodeBlock code={chain.file(main)} file={main} />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/04-control-flow.md" label="spec 4.9 · where a loop runs" />
        <SrcRef file="docs/spec/15-gpu.md" label="spec 15.2 · running on the GPU" />
        <SrcRef file="website/content/samples/guides/longest-chain.project/runs" label="the commands this page runs" />
      </div>
    </>
  ),
};
