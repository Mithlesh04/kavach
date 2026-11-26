import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { guideProject } from "./guide-parts";
import { H2, Note, P } from "./parts";

// ---------- Guides › Test with Contracts and Generated Tests ----------
//
// One task from start to finish: splitting a bill so no cent is lost.
// guides/split-first.project is the first version, which loses cents; its
// test run finds the smallest bill that shows it. guides/split-bill.project
// is the fix, with the found case in tests/found/, which `--keep` turns
// into an example.

const first = guideProject("split-first", ["src/main.kav"]);
const fixed = guideProject("split-bill", ["src/main.kav", "kavach.toml"]);
const main = "src/main.kav";

export const guideTestDoc: LessonDoc = {
  lead: "One task from start to finish: split a bill between people so no cent is lost. Say what must be true as contracts, and kavach test checks them against hundreds of generated bills, finds the smallest that breaks one, and keeps it as an example once it is fixed.",
  simple: <>Instead of writing many tests by hand, you write down the rules: everyone pays about the same, and the shares add up to the bill. Kavach then tries lots of bills, looking for one that breaks a rule.</>,
  minutes: 12,
  before: ["errors/contracts", "errors/contracts/ensure"],
  body: (
    <>
      <H2>1. Say what a bill can be</H2>
      <P>
        A bill is a total in cents and a number of people. Its <C>invariant</C>s say what makes sense: no negative
        total, at least one person. A <C>Bill</C> that breaks one can&apos;t be made, and the tests never generate one:
      </P>
      <CodeBlock code={fixed.piece(main, "type Bill")} file={main} />
      <H2>2. Say what the answer must be</H2>
      <P>
        Before writing the body, write the rules the shares must follow, as <C>ensure</C>s on the result: one share
        each, no cent lost, nobody paying more than a cent more than anyone else. The first version divides the
        total and gives everyone the same, with an example that works:
      </P>
      <CodeBlock code={first.piece(main, "fn shares")} file={main} />
      <H2>3. Let kavach test look for a bill that breaks it</H2>
      <P>
        The example passes. Then <C>kavach test</C> makes 200 bills, each meeting the invariants, and checks every{" "}
        <C>ensure</C> on each:
      </P>
      <Terminal session={first.said("test --seed 7")} />
      <P>
        It found that the shares don&apos;t add up, and shrank the bill to the smallest that shows it: 1 cent for 2
        people. Each pays <C>1 / 2</C>, which is 0, and the cent is lost. No hand-written test had a total that
        doesn&apos;t divide evenly. The case is saved in <C>tests/found/</C> and tried on every run after.
      </P>
      <H2>4. Fix it</H2>
      <P>Give the cents left over to the first people, one each:</P>
      <CodeBlock code={fixed.piece(main, "fn shares")} file={main} />
      <Terminal session={fixed.said("run src/main.kav")} />
      <P>Every generated bill now passes, the saved one included:</P>
      <Terminal session={fixed.said("test --seed 7")} />
      <H2>5. Keep the case as an example</H2>
      <P>
        <C>--keep</C> turns each saved case that now passes into an <C>example</C> on the function, and removes it from{" "}
        <C>tests/found/</C>. The bug is now part of the function&apos;s documentation, and checked on every test run:
      </P>
      <Terminal session={fixed.said("test src/main.kav --keep")} />
      <Note>
        Four lines of contracts did the work of many hand-written tests, and they stay true when the body changes:
        rewrite <C>shares</C> however you like, and the same rules check it. See{" "}
        <Link href="/docs/errors/contracts/how-they-are-checked/">How they are checked</Link>.
      </Note>
      <H2>The whole program</H2>
      <CodeBlock code={fixed.file(main)} file={main} />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/07-contracts.md" label="spec 7 · contracts" />
        <SrcRef file="docs/spec/11-testing.md" label="spec 11 · testing" />
        <SrcRef file="website/content/samples/guides/split-first.project/runs" label="the first run" />
        <SrcRef file="website/content/samples/guides/split-bill.project/runs" label="the commands after the fix" />
      </div>
    </>
  ),
};
