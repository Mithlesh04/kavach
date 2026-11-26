import Link from "next/link";
import { CodeBlock } from "@/components/code-block";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import { Terminal } from "@/components/terminal";
import type { LessonDoc } from "@/lib/chapter";
import { guideProject } from "./guide-parts";
import { H2, Note, P } from "./parts";

// ---------- Guides › Data Work with Tensors ----------
//
// One task from start to finish: guides/sensor-trends.project reads hourly
// temperatures of three sensors from a CSV into a matrix, repairs a broken
// reading with masks and `where`, and fits each sensor's trend by least
// squares with `@` and `solve`.

const sensors = guideProject("sensor-trends", ["src/main.kav", "readings.csv", "kavach.toml"]);
const main = "src/main.kav";

export const guideTensorsDoc: LessonDoc = {
  lead: "One task from start to finish: read hourly temperatures from three sensors into a matrix, find and repair a broken reading, and work out how fast each sensor warms, with masks, broadcasting and least squares, as NumPy users would.",
  simple: <>A tensor is a grid of numbers you work on all at once: &quot;subtract 5 from every reading&quot; is one line, not a loop. Here the grid is hours down and sensors across.</>,
  minutes: 14,
  before: ["standard-library/tensors", "standard-library/tensors/masks"],
  body: (
    <>
      <P>
        Three sensors write a temperature every hour. One of them failed at hour 2 and wrote <C>-99</C>, as many
        sensors do:
      </P>
      <CodeBlock code={sensors.file("readings.csv")} file="readings.csv" lang="text" />
      <H2>1. From a CSV to a matrix</H2>
      <P>
        Read the rows as a type, as in <Link href="/docs/guides/read-and-write-data-files/">Read and Write Data
        Files</Link>, then make a matrix: one row per hour, one column per sensor.
      </P>
      <CodeBlock code={sensors.piece(main, "type Reading")} file={main} />
      <CodeBlock code={sensors.piece(main, "fn main")} file={main} />
      <H2>2. Find and repair the broken reading</H2>
      <P>
        <C>t &lt; -50.0</C> compares every number at once and gives a mask, a tensor of <C>Bool</C>s.{" "}
        <C>tensor.where</C> picks from two tensors by a mask. Each sensor&apos;s mean leaves the broken readings out:
        a sum and a count along axis 0, the hours. Then the broken readings take their sensor&apos;s mean:
      </P>
      <CodeBlock code={sensors.piece(main, "fn repair")} file={main} />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>sum(axis: 0)</C> adds down each column, so it gives one number per sensor.</li>
        <li><C>means.reshape([1, 3])</C> is one row, so <C>where</C> broadcasts it down every hour: each column gets its own sensor&apos;s mean.</li>
        <li>The example checks it on one column: the broken middle reading becomes the mean of the other two.</li>
      </ul>
      <H2>3. How fast does each sensor warm?</H2>
      <P>
        Fit a straight line <C>a + b × hour</C> to each sensor, by least squares: solve{" "}
        <C>(AᵀA) x = Aᵀt</C>, where <C>A</C> has a column of ones and a column of hours. <C>@</C> is the matrix
        product. One <C>solve</C> fits all three sensors at once, one column each, and row 1 holds the slopes:
      </P>
      <CodeBlock code={sensors.piece(main, "fn trends")} file={main} />
      <P>
        <C>solve</C> fails with <C>Singular</C> when there is no single answer, such as when every reading is from the
        same hour, so <C>trends</C> says it can fail. Its example uses <C>near</C>, as a float worked out this way may
        differ in the last digit.
      </P>
      <H2>4. Run it</H2>
      <Terminal session={sensors.said("check")} />
      <Terminal session={sensors.said("run src/main.kav")} />
      <P>
        North and roof warm by exactly 0.5 and 1 degree an hour. South comes out at 0.49, not 0.5: its broken reading
        took the sensor&apos;s mean, 22.3, where the line would have put 22.0. Filling a gap with a mean is quick, and
        it bends a trend a little; what to fill with is a choice about the data, not about the code.
      </P>
      <H2>5. Test it</H2>
      <Terminal session={sensors.said("test --seed 7")} />
      <Note>
        <C>kavach test</C> can&apos;t make up tensors yet, and says so for <C>repair</C> and <C>trends</C>, so their
        examples are what check them.
      </Note>
      <H2>The whole program</H2>
      <CodeBlock code={sensors.file(main)} file={main} />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/14-standard-library/33-tensor.md" label="spec 14.33 · std.tensor" />
        <SrcRef file="website/content/samples/guides/sensor-trends.project/runs" label="the commands this page runs" />
      </div>
      <P>
        More on each step: <Link href="/docs/standard-library/tensors/masks/">Masks</Link>,{" "}
        <Link href="/docs/standard-library/tensors/reductions/">Reductions</Link>,{" "}
        <Link href="/docs/standard-library/tensors/arithmetic-and-broadcasting/">Broadcasting</Link> and{" "}
        <Link href="/docs/standard-library/tensors/linear-algebra/">Linear algebra</Link>.
      </P>
    </>
  ),
};
