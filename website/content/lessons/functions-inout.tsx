import Link from "next/link";
import { C } from "@/components/inline-code";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, P } from "./parts";

// ---------- inout Parameters ----------

export const inoutDoc: LessonDoc = {
  lead: "A parameter marked inout lets the function change the caller's variable, and only when it succeeds.",
  simple: <>Usually a function gets a copy of your value. With <C>inout</C>, it gets the value itself and may change it, like lending someone your notebook instead of a photocopy.</>,
  minutes: 11,
  before: ["functions/functions"],
  body: (
    <>
      <Ex name="functions/sub-inout" />
      <P>
        The call looks ordinary, <C>bump(count)</C>, and the function changes its own copy; when it returns, the
        new value goes back into <C>count</C>. The variable must be a <C>let</C>:
      </P>
      <Ex name="functions/inout-const.fails" />
      <H2>When the function fails after changing it</H2>
      <P>
        Here <C>withdraw</C> takes the money out first and checks afterwards. The second call fails after{" "}
        <C>balance</C> has already gone down to -80 inside it, and the caller handles the error with{" "}
        <C>catch</C>. What does the caller&apos;s <C>balance</C> hold now?
      </P>
      <Ex name="functions/inout-fail" />
      <P>
        Still <strong className="text-ink">20</strong>. The function changes its own copy, and the copy goes back to
        the caller only when the function succeeds. A failed call leaves the caller&apos;s variable exactly as it
        was: never half-changed.
      </P>
      <H2>Fields and list items</H2>
      <P>Anything the caller could assign to works: an item of a list, or a field of a record held in a <C>let</C>.</P>
      <Ex name="functions/inout-places" />
      <H2>Passing it on</H2>
      <P>
        An <C>inout</C> parameter is itself a place, so a function can pass it on as <C>inout</C> to another call, a
        recursive one too. Every change reaches the first caller&apos;s variable:
      </P>
      <Ex name="functions/inout-pass-on" />
      <P><C>checkout</C> adds 10% tax, then a fee of 5: 100, then 110, then 115. <C>count_down</C> adds 4, 3, 2 and 1 through its own recursive calls.</P>
      <H2>Not into a callback</H2>
      <P>
        A lambda can&apos;t change an outside variable, an <C>inout</C> parameter included: it keeps a copy, so a
        change inside it doesn&apos;t reach the outside, and the compiler warns. A callback can be stored and called
        later, or run in another task, after the function that owns the variable has returned. A call finishes
        before its caller goes on, so passing the place to a call is safe.
      </P>
      <Ex name="functions/inout-lambda.warns" />
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">You want</th>
              <th className="py-2 font-bold">Write</th>
            </tr>
          </thead>
          <tbody className="text-muted">
            <tr className="border-b border-line align-top"><td className="py-2 pr-4 text-ink">Add up items</td><td className="py-2"><C>total += items.sum(x =&gt; x)</C>: the callback gives back a value</td></tr>
            <tr className="border-b border-line align-top"><td className="py-2 pr-4 text-ink">Change it step by step</td><td className="py-2">a <C>for</C> loop: <C>total += x</C> changes the real variable</td></tr>
            <tr className="border-b border-line align-top"><td className="py-2 pr-4 text-ink">Many callbacks or tasks changing one value</td><td className="py-2"><Link href="/docs/memory/shared-state/"><C>Shared&lt;T&gt;</C></Link></td></tr>
          </tbody>
        </table>
      </div>
      <H2>Not the same variable twice</H2>
      <P>Giving one variable to two parameters of the same call would make two names for one value, so it is refused:</P>
      <Ex name="functions/inout-twice.fails" />
      <H2>Not as a function value</H2>
      <P>
        A function with an <C>inout</C> parameter can only be called by its name: it can&apos;t be stored in a
        variable or passed to another function.
      </P>
      <Ex name="functions/inout-value.fails" />
      <H2>Through an interface</H2>
      <P>
        An interface method can have <C>inout</C> parameters, and a call through an interface value changes the
        caller&apos;s variable too:
      </P>
      <Ex name="functions/inout-iface" />
      <H2>Not in a lambda</H2>
      <P>A lambda can&apos;t take an <C>inout</C> parameter. When a small function must change the caller&apos;s variable, give it a name:</P>
      <Ex name="habits/lambda-inout.fails" />
      <H2>When it panics</H2>
      <P>
        A panic is different: it is a bug, not an error, so no caller can catch it. The program stops there, and no
        variable is looked at again:
      </P>
      <Ex name="functions/inout-panic.panics" />
    </>
  ),
  deeper: (
    <P>
      Nothing is shared while the function runs, so no other part of the program can see half a change. If the
      function fails, the caller&apos;s variable keeps its old value. Giving the same variable twice in one call, as
      in <C>swap(x, x)</C>, is refused: it would be two names for one value.
    </P>
  ),
};
