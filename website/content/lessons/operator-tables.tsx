import { C } from "@/components/inline-code";
import { loadSample } from "@/lib/samples";

// The operator tables of spec 1.8, in plain words. The Result column is
// never typed: it is the output of a sample that prints one line per row,
// in row order, run by the real kavach (scripts/check-samples.mjs). The
// build fails if a row's example isn't in its sample, or the counts differ.

type Row = [op: string, name: string, meaning: string, example: string];

export const operatorGroups: {
  title: string;
  sample: string;
  rows: Row[];
  resultNote?: string;
  /** Chains: the example is the operator, so show only Chain | Means | Result. */
  chain?: boolean;
  /** Header of the first column, and of the result column. */
  formHeader?: string;
  resultHeader?: string;
}[] = [
  {
    title: "Arithmetic",
    sample: "operators/t-arithmetic",
    rows: [
      ["+", "Addition", "Adds two numbers", "7 + 2"],
      ["-", "Subtraction", "Takes the second number from the first", "7 - 2"],
      ["*", "Multiplication", "Multiplies two numbers", "7 * 2"],
      ["/", "Division", "Divides; two whole numbers give a whole number, the fraction cut off", "7 / 2"],
      ["/", "Division", "With floats, the exact result", "7.0 / 2.0"],
      ["%", "Remainder", "What is left over after dividing", "7 % 2"],
      ["**", "Power", "Raises the first number to the power of the second", "2 ** 10"],
      ["-", "Negation", "Turns a number to its opposite sign", "-(7 + 2)"],
    ],
  },
  {
    title: "Comparison",
    sample: "operators/t-comparison",
    rows: [
      ["==", "Equal to", "True when both sides are the same", "5 == 5"],
      ["!=", "Not equal to", "True when the sides differ", "5 != 3"],
      ["<", "Less than", "True when the left is smaller", "3 < 5"],
      ["<=", "Less than or equal to", "True when the left is smaller or the same", "5 <= 5"],
      [">", "Greater than", "True when the left is bigger", "7 > 9"],
      [">=", "Greater than or equal to", "True when the left is bigger or the same", "9 >= 9"],
      ["a < b < c", "Chain", "Both comparisons at once, as in mathematics", "1 < 5 < 10"],
    ],
  },
  {
    title: "Chained comparisons",
    sample: "operators/t-chain",
    chain: true,
    resultNote: "Results with a = 5, b = 5, c = 3, d = 7, e = 2, f = 9, g = 9.",
    rows: [
      ["== !=", "Three values", "a == b and b != c", "a == b != c"],
      ["< <=", "Three values", "c < a and a <= b", "c < a <= b"],
      ["== != <", "Four values", "a == b and b != c and c < d", "a == b != c < d"],
      ["== != < >", "Five values", "a == b and b != c and c < d and d > e", "a == b != c < d > e"],
      ["< < == <", "Any order", "e < c and c < a and a == b and b < d", "e < c < a == b < d"],
      ["!= ==", "Only neighbours", "a != b and b == c: a and c are never compared", "a != b == c"],
      ["!= !=", "Only neighbours", "a != c and c != b: true, although a and b are equal", "a != c != b"],
      ["< < == < < ==", "Seven values", "e < c and c < a and a == b and b < d and d < f and f == g", "e < c < a == b < d < f == g"],
      ["( ) != ( )", "Grouped", "Each bracket is worked out first, to true or false; then the two answers are compared", "(a == b) != (c == d)"],
      ["( ) == ( )", "Grouped", "true == true: both brackets give the same answer", "(c < a) == (e < d)"],
      ["( ) == ( ) == ( )", "Grouped chain", "Three answers, compared as a chain of their own", "(a < b) == (d > f) == (e > c)"],
      ["or", "Chains with or", "True when either chain is true; here neither is", "a == b == c or d == e == d"],
      ["or", "Chains with or", "The first chain is true, so the whole is true", "a == b == 5 or d == e == d"],
      ["and", "Chains with and", "(c < a == b) and (d > e < c): both chains must be true", "c < a == b and d > e < c"],
      ["not", "A chain turned around", "a == b == c is false, so not makes it true", "not (a == b == c)"],
    ],
  },
  {
    title: "Logical",
    sample: "operators/t-logical",
    rows: [
      ["and", "And", "True only when both sides are true", "true and false"],
      ["or", "Or", "True when at least one side is true", "true or false"],
      ["not", "Not", "Turns true into false, and false into true", "not true"],
    ],
  },
  {
    title: "Assignment",
    sample: "operators/t-assignment",
    resultNote: "Each result is x afterwards, starting from 10.",
    rows: [
      ["=", "Assign", "Gives the variable a new value", "x = 4"],
      ["+=", "Add and assign", "x += 3 is x = x + 3", "x += 3"],
      ["-=", "Subtract and assign", "x -= 3 is x = x - 3", "x -= 3"],
      ["*=", "Multiply and assign", "x *= 3 is x = x * 3", "x *= 3"],
      ["/=", "Divide and assign", "x /= 3 is x = x / 3", "x /= 3"],
      ["%=", "Remainder and assign", "x %= 3 is x = x % 3", "x %= 3"],
      ["**=", "Power and assign", "x **= 3 is x = x ** 3", "x **= 3"],
    ],
  },
  {
    title: "Optional values",
    sample: "operators/t-optional",
    rows: [
      ["??", "Default", "The value if there is one, otherwise the one on the right", "nickname ?? \"friend\""],
      ["?.", "Read through", "Reads a field when the value is there, and gives none when it isn't", "found?.name"],
    ],
  },
  {
    title: "Range",
    sample: "operators/t-range",
    resultNote: "Each result is the numbers a for loop over it visits.",
    rows: [
      ["..", "Range", "From the first number up to, but not including, the last", "0..3"],
      ["..=", "Range, end included", "From the first number up to and including the last", "0..=3"],
    ],
  },
  {
    title: "Bits",
    sample: "operators/t-bits",
    rows: [
      ["&", "And", "Keeps the bits set in both numbers", "12 & 10"],
      ["|", "Or", "Keeps the bits set in either number", "12 | 10"],
      ["^", "Exclusive or", "Keeps the bits set in exactly one of the numbers", "12 ^ 10"],
      ["~", "Not", "Flips every bit", "~0"],
      ["<<", "Shift left", "Moves the bits left: each step doubles the number", "1 << 4"],
      [">>", "Shift right", "Moves the bits right: each step halves the number, keeping its sign", "32 >> 2"],
    ],
  },
];

/** One group's table: the operator, its name, what it does, an example, and the real result. */
export function OpTable({
  title,
  sample,
  rows,
  resultNote,
  chain,
  formHeader = "Operator",
  resultHeader = "Result",
}: (typeof operatorGroups)[number]) {
  const s = loadSample(sample);
  const results = (s.output ?? "").split("\n");
  if (results.length !== rows.length) {
    throw new Error(`operator table "${title}": ${rows.length} rows but ${sample} prints ${results.length} lines`);
  }
  let from = 0;
  for (const [, , , example] of rows) {
    const at = s.code.indexOf(example, from);
    if (at < 0) throw new Error(`operator table "${title}": "${example}" is not in ${sample}, in row order`);
    from = at + example.length;
  }
  const th = "py-2 pr-3 font-bold";
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[15px]">
        <caption className="mb-2 text-left">
          <span className="block text-lg font-extrabold text-ink">{title}</span>
          {resultNote ? <span className="block text-sm text-soft">{resultNote}</span> : null}
        </caption>
        <thead>
          <tr className="border-b border-line text-left text-soft">
            {chain ? (
              <>
                <th className={th}>{formHeader === "Operator" ? "Chain" : formHeader}</th>
                <th className={th}>Means</th>
              </>
            ) : (
              <>
                <th className={th}>{formHeader}</th>
                <th className={th}>Name</th>
                <th className={th}>What it does</th>
                <th className={th}>Example</th>
              </>
            )}
            <th className="py-2 font-bold">{resultHeader}</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(([op, name, meaning, example], i) => (
            <tr key={example} className="border-b border-line align-top">
              {chain ? (
                <>
                  <td className="py-2 pr-3 whitespace-nowrap"><C>{example}</C></td>
                  <td className="py-2 pr-3 text-muted">
                    <span className="font-semibold text-ink">{name}:</span> {meaning}
                  </td>
                </>
              ) : (
                <>
                  <td className="py-2 pr-3 whitespace-nowrap"><C>{op}</C></td>
                  <td className="py-2 pr-3 whitespace-nowrap font-semibold text-ink">{name}</td>
                  <td className="py-2 pr-3 text-muted">{meaning}</td>
                  <td className="py-2 pr-3 whitespace-nowrap"><C>{example}</C></td>
                </>
              )}
              <td className="py-2 whitespace-nowrap"><C>{results[i]}</C></td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}