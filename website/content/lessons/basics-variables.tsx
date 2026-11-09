import { C } from "@/components/inline-code";
import type { LessonDoc } from "@/lib/chapter";
import { OpTable } from "./operator-tables";
import { Ex, H2, Note, P } from "./parts";

// ---------- Variables ----------

// The naming rules of spec 1.4 and 1.5; the samples below the table show the main ones run or refused.
const naming: [string, React.ReactNode, React.ReactNode][] = [
  ["Letters, digits and _", <C key="a">player_2</C>, <C key="b">player-2</C>],
  ["Can't start with a digit", <C key="a">place_2nd</C>, <C key="b">2nd_place</C>],
  ["Any script", <C key="a">नाम</C>, <span key="b" className="text-muted">mixing scripts in one name (a warning)</span>],
  ["snake_case for values", <C key="a">total_price</C>, <C key="b">totalPrice</C>],
  ["Not a keyword", <C key="a">matched</C>, <C key="b">match</C>],
  ["Once per block", <span key="a" className="text-muted">the same name in an inner block</span>, <span key="b" className="text-muted">the same name twice in one block</span>],
  ["Case counts", <span key="a" className="text-muted"><C>total</C> and <C>Total</C> are different names</span>, <span key="b" className="text-muted">(a value&apos;s name is snake_case anyway)</span>],
];

export const variablesDoc: LessonDoc = {
  lead: "A variable is a name for a value. const names one that never changes; let names one that can.",
  simple: <>A variable is like a labelled jar: the label is the name, and what is inside is the value. Some jars are sealed (<C>const</C>), and some you can open and refill (<C>let</C>).</>,
  minutes: 4,
  before: ["basics/output"],
  body: (
    <>
      <OpTable
        title="Every form, with what it gives"
        sample="variables/t-forms"
        formHeader="Form"
        resultHeader="Prints"
        resultNote="Each example is followed by a print of the name it sets."
        rows={[
          ["const name = value", "Constant", "Names a value that never changes", 'const city = "Pune"'],
          ["let name = value", "Variable", "Names a value that can change later", "let count = 3"],
          ["name = value", "Change", "Gives a let a new value", "count = count + 1"],
          ["name += value", "Change in place", "Short for count = count + 10", "count += 10"],
          ["const name: Type = value", "With a type", "The same, with the type written out", "const limit: Int = 10"],
          ["const [a, b] = list", "Take apart", "Several names from one list, by position", "const [first, second] = [7, 8]"],
        ]}
      />
      <H2>A whole program</H2>
      <Ex name="values/sub-types" />
      <P>
        You don&apos;t have to write a type: <C>31</C> makes <C>age</C> an <C>Int</C>, and <C>&quot;asha&quot;</C>{" "}
        makes <C>name</C> a <C>String</C>. Write one when you want to say it, as in <C>price: Float</C>.
      </P>
      <Note>
        Use <C>const</C> unless the value really must change. Then a reader knows at a glance which names can move.
      </Note>
      <H2>Naming rules</H2>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-[15px]">
          <thead>
            <tr className="border-b border-line text-left text-soft">
              <th className="py-2 pr-4 font-bold">Rule</th>
              <th className="py-2 pr-4 font-bold">Allowed</th>
              <th className="py-2 font-bold">Not allowed</th>
            </tr>
          </thead>
          <tbody>
            {naming.map(([rule, ok, bad]) => (
              <tr key={rule} className="border-b border-line align-top">
                <td className="py-2 pr-4 text-muted">{rule}</td>
                <td className="py-2 pr-4">{ok}</td>
                <td className="py-2">{bad}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <Ex name="variables/names-ok" />
      <P>A name can&apos;t start with a digit:</P>
      <Ex name="variables/digit.fails" />
      <P>A keyword can&apos;t be a name either, and the message suggests one:</P>
      <Ex name="habits/keyword.fails" />
      <P>Nor can it be declared twice in the same block. An inner block may reuse it, and the outer name comes back after:</P>
      <Ex name="variables/twice.fails" />
      <Ex name="variables/scope" />
      <H2>Names that start with _</H2>
      <P>
        A variable nobody reads gets a warning, so leftovers don&apos;t pile up. Start the name with <C>_</C> to say
        it is unused on purpose, and the warning goes. <C>_</C> on its own throws a value away: an item you skip, or
        a loop counter you don&apos;t need.
      </P>
      <Ex name="variables/underscore.warns" />
    </>
  ),
  tryIt: {
    task: <>Make a <C>let</C> named <C>count</C> starting at 0, add 1 to it twice, and print it.</>,
    answer: <Ex name="variables/let" />,
  },
  quiz: [
    {
      type: "choice",
      prompt: <>What happens when you run this program?</>,
      code: "const price = 40\nprice = price + 2\nprint(price)",
      options: [
        "It prints 42",
        "It prints 40",
        "It doesn't run: the compiler refuses it, because price is a const",
      ],
      answer: 2,
      explain: "price is a const, so it can't change. The compiler stops at the second line, before anything runs, and says to declare it with let.",
    },
  ],
};

export const constDoc: LessonDoc = {
  lead: "const names a value that never changes after it is made.",
  simple: <>A <C>const</C> is a value written in ink: once it is there, it stays. Use it for anything that should not change, such as a person&apos;s date of birth.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="variables/const" />
      <P>Changing a <C>const</C> is refused before the program runs, with the fix in the message:</P>
      <Ex name="values/sub-let.fails" />
    </>
  ),
};

export const letDoc: LessonDoc = {
  lead: "let names a variable: a value that can change later.",
  simple: <>A <C>let</C> is a value written in pencil: you can rub it out and write a new one, like a score that goes up during a game.</>,
  minutes: 2,
  body: (
    <>
      <Ex name="variables/let" />
      <P>
        Kavach has no <C>var</C>. Writing it is an error that tells you what to write instead:
      </P>
      <Ex name="variables/var.fails" />
    </>
  ),
};

export const changeDoc: LessonDoc = {
  lead: "Give a let a new value with =, or change it in place with +=, -=, *= and the others.",
  simple: <>Changing a <C>let</C> is like updating a scoreboard: <C>score = 10</C> writes a new number, and <C>score += 5</C> adds five to what is already there.</>,
  minutes: 3,
  body: (
    <>
      <Ex name="variables/change" />
      <P>
        <C>score *= 2</C> is short for <C>score = score * 2</C>, and is checked the same way: a result too big for
        an <C>Int</C> stops the program with a message instead of turning into a wrong number.
      </P>
      <H2>let makes a new one</H2>
      <P>
        Writing <C>let</C> again inside a loop doesn&apos;t change the variable outside it: it makes a new one, which
        is gone at the end of each turn. The total stays <C>0</C>, and the compiler says why:
      </P>
      <Ex name="variables/hiding-let.warns" />
      <P>
        Write <C>total = total + i</C>, without <C>let</C>, and it prints <C>3</C>. The editor offers that change as
        the quick fix &quot;Change the <C>total</C> declared above&quot;. You might have meant a new variable after
        all, so <C>kavach fix</C> never makes it for you.
      </P>
    </>
  ),
};

export const destructuringDoc: LessonDoc = {
  lead: "Take a list, a record or a map apart into several names in one line.",
  simple: <>Like unpacking a bag onto a table, one line takes a value apart and gives each piece its own name, so you don&apos;t have to pick out the pieces one by one.</>,
  minutes: 5,
  body: (
    <>
      <Ex name="values/sub-apart" />
      <ul className="m-0 flex list-disc flex-col gap-1.5 pl-5">
        <li>A list is taken apart by position. A name with no item left gets <C>none</C>, as <C>third</C> does.</li>
        <li><C>...rest</C> takes all the items after the others, as a list.</li>
        <li>A record is taken apart by field name; <C>age: years</C> gives the field another name.</li>
        <li><C>= &quot;8080&quot;</C> is a default, used when the value is missing.</li>
        <li><C>_</C> skips an item you don&apos;t need.</li>
      </ul>
    </>
  ),
};
