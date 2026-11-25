// A tight `for` loop over integers: 500 million rounds, on all cores.
// Each range keeps its own sum modulo 1,000,000,007, and the sums are
// added modulo the same number: no part is negative, so the result is
// the one-core loop's, as Kavach's `on cores` gives it.
using System.Collections.Concurrent;
using System.Threading.Tasks;

const long M = 1000000007;
long total = 0;
object gate = new object();
Parallel.ForEach(Partitioner.Create(0L, 500000000L), () => 0L, (range, state, part) =>
{
    for (long i = range.Item1; i < range.Item2; i++)
    {
        part = (part + i * 7 % 13) % M;
    }
    return part;
}, part =>
{
    lock (gate) { total = (total + part) % M; }
});
System.Console.WriteLine(total);
