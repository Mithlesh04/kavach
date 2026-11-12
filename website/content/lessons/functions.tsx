import Link from "next/link";
import { C } from "@/components/inline-code";
import { SrcRef } from "@/components/src-ref";
import type { LessonDoc } from "@/lib/chapter";
import { Ex, H2, Note, P, Syntax } from "./parts";

// ---------- Functions ----------

export const functionsDoc: LessonDoc = {
  lead: "A function is a named block of code. Write it once, then run it as often as you like by calling its name.",
  simple: <>A function is like a recipe: write the steps once, give them a name, and follow them whenever you need the dish, as often as you like.</>,
  minutes: 5,
  before: ["basics/variables", "basics/if-else"],
  body: (
    <>
      <H2>Syntax</H2>
      <Syntax code={"fn name(parameter: Type) -> ResultType {\n    // the last line is the result\n}"} />
      <H2>Example</H2>
      <Ex name="functions/sub-last" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>fn</C> starts a function, then its name and its parameters in brackets.</li>
        <li><C>-&gt; String</C> says what the function gives back.</li>
        <li>The last line of the body is the result. There is no <C>return</C> to forget.</li>
      </ul>
      <Note>
        Parameters and the result always have their types written. Reading the first line of a function tells you
        everything it takes and gives.
      </Note>
    </>
  ),
  tryIt: {
    task: <>Write <C>fn square(n: Int) -&gt; Int</C> that gives <C>n</C> times <C>n</C>, and print <C>square(9)</C>.</>,
    answer: <P>The body is one line, <C>n * n</C>: being the last line, it is the result. <C>square(9)</C> is <C>81</C>.</P>,
  },
  quiz: [
    {
      type: "type-output",
      prompt: <>What does this print?</>,
      code: 'fn add(a: Int, b: Int) -> Int {\n    a + b\n}\n\nprint(add(2, 3))',
      answer: "5",
      explain: "The last line, a + b, is the result.",
    },
  ],
};

export const declaringDoc: LessonDoc = {
  lead: "fn name() { … } declares a function; name() calls it.",
  simple: <>Declaring a function is writing the recipe down and giving it a name. Calling it is cooking from it: you say its name, and its steps run.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="functions/declare" />
      <P>
        A function with no <C>-&gt;</C> gives nothing back: it is called for what it does, here printing. It can be
        written above or below the code that calls it.
      </P>
    </>
  ),
};

export const parametersDoc: LessonDoc = {
  lead: "Parameters are the values a function takes, each with a name and a type.",
  simple: <>Parameters are the ingredients a recipe asks for. Each one has a name and says what kind of thing it must be, so nobody puts sugar where the salt goes.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="functions/params" />
      <P>A parameter is like a <C>const</C>: the function can&apos;t change it. Trying to is refused, with the fix:</P>
      <Ex name="functions/param-const.fails" />
    </>
  ),
};

export const returningDoc: LessonDoc = {
  lead: "The last line is the result; return leaves early with a result.",
  simple: <>A function can hand something back when it is done, like a machine that takes coins and gives you a ticket. The last line is what it gives back.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="functions/sub-early" />
      <P>
        <C>return</C> is for leaving early, as for the invalid score here. A function without <C>-&gt;</C>{" "}
        can&apos;t end with a value, because it would be thrown away; the compiler says what to add:
      </P>
      <Ex name="functions/no-arrow.fails" />
    </>
  ),
};

// ---------- Named Arguments ----------

export const namedDoc: LessonDoc = {
  lead: "Any argument can be passed by its name, and a parameter can have a default value.",
  simple: <>Instead of remembering which value goes first, you can say what each one is for, like filling in a form with labelled boxes. Some boxes come already filled in.</>,
  minutes: 3,
  before: ["functions/functions"],
  body: (
    <>
      <Ex name="functions/sub-named" />
      <P>
        <C>greeting</C> has a default, so a call may leave it out. Naming an argument says what it means at the
        place of the call: <C>greet(&quot;Asha&quot;, greeting: &quot;Namaste&quot;)</C>.
      </P>
    </>
  ),
};

export const namedArgsDoc: LessonDoc = {
  lead: "Name the arguments, and their order no longer matters.",
  simple: <>Saying <C>price: 40, count: 3</C> is like labelling boxes before you hand them over: the function knows which is which, in any order.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="functions/named-order" />
      <P>Arguments given by position come first; named ones follow, in any order.</P>
    </>
  ),
};

export const defaultsDoc: LessonDoc = {
  lead: "A parameter with = value may be left out of a call, and then takes that value.",
  simple: <>A default is the usual choice, filled in for you, like a coffee shop that adds milk unless you ask for it black.</>,
  minutes: 2,
  body: <Ex name="functions/sub-named" />,
};

// ---------- Using Results ----------

export const resultsDoc: LessonDoc = {
  lead: "A result must be used. To throw one away on purpose, write const _ = …",
  simple: <>If you ask a question, you should listen to the answer. When a function gives something back, Kavach makes you use it, so a forgotten answer can&apos;t hide a mistake.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="functions/sub-unused.fails" />
      <P>
        A forgotten result is a common bug: making an updated copy of a value and never using it. When you really
        don&apos;t need the result, say so:
      </P>
      <Ex name="functions/discard" />
    </>
  ),
};

// ---------- Lambdas ----------

export const lambdasDoc: LessonDoc = {
  lead: "A lambda is a small function without a name, written where it is used: x => x * 2.",
  simple: <>A lambda is a small, nameless function written right where you need it, like a sticky note with one instruction on it: &quot;double it&quot;.</>,
  minutes: 4,
  before: ["functions/functions"],
  body: (
    <>
      <Ex name="functions/sub-lambda" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>x =&gt; x * 2</C> takes <C>x</C> and gives <C>x * 2</C>. Its types come from where it is used.</li>
        <li>Lambdas shine as arguments: <C>prices.map(p =&gt; p + 10)</C> adds 10 to every price.</li>
        <li>A lambda keeps a copy of the outside values it uses: changing <C>limit</C> later doesn&apos;t change <C>over</C>.</li>
      </ul>
      <P>
        Because it is a copy, a lambda that changes an outside variable changes only its own copy. Other languages
        would print 2 here; Kavach prints 0, and warns where it happens:
      </P>
      <Ex name="functions/lambda-copy.warns" />
      <P>
        A change to the copy, with <C>=</C>, <C>+=</C> or a method that changes a value (<C>push</C>, <C>add</C>,{" "}
        <C>remove</C>), lasts only for that call: each call starts again from the copy taken when the lambda was
        made. So each sum below is just its own item, each call&apos;s log is <C>[0]</C> and one item, and the real{" "}
        <C>total</C> and <C>log</C> never change:
      </P>
      <Ex name="functions/lambda-change.warns" />
      <P>To count from inside, give the new value back from the lambda, or share it on purpose with <C>Shared</C>.</P>
      <H2>try inside a lambda</H2>
      <P>
        A <C>try</C> or <C>fail</C> inside a lambda belongs to the lambda: it ends that call of the lambda with the
        error, so the lambda is a function that can fail. The error never jumps out of the function the lambda was
        written in, since the lambda may be called long after that function has returned.
      </P>
      <Ex name="functions/lambda-try" />
      <P>
        Each call of <C>doubled</C> is a call that can fail, so it gets a <C>catch</C>. Given to a method such as{" "}
        <C>map</C>, such a lambda makes the <C>map</C> call the one that can fail:{" "}
        <Link href="/docs/errors/try/">try</Link> shows how.
      </P>
    </>
  ),
};

export const writingLambdaDoc: LessonDoc = {
  lead: "One parameter: x => …. Several: (a, b) => …. A longer body goes in braces.",
  simple: <>On the left of the arrow are the values the note receives; on the right, what to do with them. Longer instructions go between braces.</>,
  minutes: 2,
  body: (
    <>
      <Syntax code={"x => x * 2\n(a, b) => a + b\nmsg => {\n    debug(msg)\n    msg.len()\n}"} />
      <Ex name="functions/sub-lambda" />
    </>
  ),
};

export const dotDoc: LessonDoc = {
  lead: "In an argument, .something is short for a lambda over one value: .upper() means x => x.upper().",
  simple: <><C>.upper()</C> is a shorter way to say &quot;take each one and make it upper case&quot;, for when the instruction is that simple.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="functions/dot" />
      <P>
        <C>.paid</C> reads the <C>paid</C> field of each order, and <C>.len() &gt; 4</C> asks of each name whether
        it is longer than 4. No parameter to name, nothing to misread.
      </P>
    </>
  ),
};

// ---------- Functions as Values ----------

export const fnValuesDoc: LessonDoc = {
  lead: "A function is a value: store it in a variable, or pass it to another function.",
  simple: <>A function can be handed around like any other thing: kept in a variable, or given to another function that decides when to use it, like handing a recipe to a friend.</>,
  minutes: 5,
  before: ["functions/lambdas"],
  body: (
    <>
      <Ex name="functions/values" />
      <P>
        <C>fn(Int, Int) -&gt; Int</C> is the type of a function that takes two <C>Int</C>s and gives one.{" "}
        <C>apply_twice</C> takes such a function as its parameter <C>f</C> and calls it.
      </P>
      <H2>Functions that can fail</H2>
      <P>
        A function type says what its functions can fail with, and what they may use, in the order of a signature:{" "}
        <C>fn(NewOrder) -&gt; Order ! ShopError uses Db</C>. Several capabilities go in parentheses:{" "}
        <C>uses (Net, Db)</C>.
      </P>
      <Ex name="functions/fn-errors" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li><C>handle</C> takes any function that can fail. Its error type <C>E</C> is a type parameter, so <C>handle</C> fails with <C>place</C>&apos;s own <C>ShopError</C>, and the <C>match</C> on it must cover every case.</li>
        <li>Calling a function value that can fail is a call that can fail: <C>try</C>, <C>catch</C> or <C>match</C>, as for any other call.</li>
        <li>A function that can&apos;t fail fits any function type, with <C>!</C> or without.</li>
        <li>With <C>uses</C>, a function value may use only what its type lists; without it, the type sets no limit. See <Link href="/docs/agents/capabilities/pure-functions/">Pure Functions</Link>.</li>
      </ul>
      <P>A function that can fail doesn&apos;t fit a type without <C>!</C>, since its error would be lost:</P>
      <Ex name="functions/fn-errors.fails" />
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="docs/spec/03-functions.md" label="spec 3.4 · function types" />
        <SrcRef file="tests/conformance/03-functions/function-types-say-their-errors.kav" label="tested" />
        <SrcRef file="docs/decisions/0085-function-types-with-errors-and-uses.md" label="decision 0085" />
      </div>
      <H2>Any number of values</H2>
      <P>
        There are no rest parameters, such as <C>...args</C> in JavaScript or <C>*args</C> in Python. A function that
        takes any number of values takes a list, and a call puts them in brackets:
      </P>
      <Ex name="functions/many-values" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>An empty list is fine: <C>total([])</C> is 0.</li>
        <li>A list you already have passes on as it is. There is no spread operator to learn, such as <C>...list</C> or <C>*list</C>.</li>
        <li>The parameter is like any other, so it mixes with named arguments and defaults without extra rules. Rust, Zig and Haskell do the same.</li>
      </ul>
      <P>Values without the brackets are refused:</P>
      <Ex name="functions/many-values.fails" />
    </>
  ),
};

// ---------- Recursion ----------

export const recursionDoc: LessonDoc = {
  lead: "A function may call itself, as long as each call gets closer to a case that stops.",
  simple: <>A function that calls itself works like a set of nesting dolls: open one, and there is a smaller one inside, until you reach the smallest, which has nothing inside.</>,
  minutes: 4,
  before: ["functions/functions"],
  body: (
    <>
      <Ex name="functions/factorial" />
      <P>
        <C>factorial(5)</C> is <C>5 * factorial(4)</C>, and so on down to <C>factorial(1)</C>, which is just{" "}
        <C>1</C>: the case that stops.
      </P>
      <H2>Too deep</H2>
      <P>Each call waits for the next, and the waiting calls take room. Far too many stop the program, cleanly:</P>
      <Ex name="functions/deep.panics" />
      <H2>How deep it can go</H2>
      <P>
        Each call that waits keeps its parameters and values on the stack, a block of memory that holds the calls
        still running. How many fit depends on where the program runs:
      </P>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Where it runs</th>
              <th className="py-2 pr-4 font-bold">Stack</th>
              <th className="py-2 font-bold">Deepest recursion</th>
            </tr>
          </thead>
          <tbody>
            {[
              ["kavach run (the interpreter)", "512 MB, of which it keeps 32 MB in hand to report the overflow", "tens of thousands of small calls, about 50,000 in a release build of kavach: measured as calls use the stack, not counted, so bigger calls go less deep"],
              ["Native code: kavach build, kavach run --native", "512 MB for the program", "about 13 million calls of a small function like depth above (measured with kavach run --native), about 36 bytes each; fewer when each call holds more values"],
              ["Each task, from spawn", "512 MB each, of address space: memory is used only as calls need it", "the same as native code"],
              ["A library called from another language (kavach build --lib)", "256 KB below the caller", "a few thousand small calls: the caller's own stack is unknown, often 1 MB, so a deep recursion stops well before it"],
              ["A gpu fn", "none", "no recursion at all: it is a compile error"],
            ].map(([where, stack, deepest]) => (
              <tr key={where} className="border-b border-line align-top">
                <td className="py-2 pr-4 text-ink">{where}</td>
                <td className="py-2 pr-4 text-muted">{stack}</td>
                <td className="py-2 text-muted">{deepest}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <P>
        Past the limit, the program stops with the panic above: never a crash or corrupted memory.
      </P>
      <Note>For a long repetition, a <Link href="/docs/basics/for-loop/">for</Link> or <Link href="/docs/basics/while-loop/">while</Link> loop is the right tool.</Note>
      <div className="flex flex-wrap gap-2.5">
        <SrcRef file="compiler/interp/src/stack.rs" line={10} symbol="STACK_USE" />
        <SrcRef file="compiler/interp/src/lib.rs" line={52} symbol="STACK_SIZE" />
        <SrcRef file="runtime/native/src/lib.rs" line={76} symbol="STACK_SIZE" />
        <SrcRef file="runtime/native/src/lib_calls.rs" line={21} symbol="LIB_STACK" />
      </div>
    </>
  ),
};

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

// ---------- Signatures ----------

export const signatureDoc: LessonDoc = {
  lead: "A function's first lines say what it takes, gives, may fail with, may touch, and promises.",
  simple: <>A function&apos;s first lines are like the label on a medicine box: what goes in, what comes out, what can go wrong, and what it may touch, all before you open it.</>,
  minutes: 4,
  before: ["functions/functions"],
  body: (
    <>
      <Ex name="functions/signature" />
      <P>
        Between the result type and the body, a function can state more, always in this order: the errors it can
        give (<C>!</C>), the capabilities it uses (<C>uses</C>), what callers must ensure (<C>require</C>), what it
        promises (<C>ensure</C>), and examples (<C>example</C>). Each has its own part of the course.
      </P>
      <Note>
        Reading only the signature tells a reviewer what a function does, without reading its body. That matters
        when someone else, or an AI, wrote the body.
      </Note>
    </>
  ),
};
