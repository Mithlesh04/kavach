// A growing list: push 20 million numbers, then add them up on all
// cores. Whole numbers add up to the same total in any order.
using System.Collections.Concurrent;
using System.Threading;
using System.Threading.Tasks;

var items = new System.Collections.Generic.List<long>();
for (long i = 0; i < 20000000; i++)
{
    items.Add(i % 1000);
}
long total = 0;
Parallel.ForEach(Partitioner.Create(0, items.Count), () => 0L, (range, state, part) =>
{
    for (int k = range.Item1; k < range.Item2; k++)
    {
        part += items[k];
    }
    return part;
}, part => Interlocked.Add(ref total, part));
System.Console.WriteLine(total);
