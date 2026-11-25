// Sorting 5 million numbers on all cores. As Kavach's sort does, one
// pass splits the numbers by their top 8 bits into 256 buckets, then
// the buckets are sorted on all cores (here with Array.Sort). The
// numbers are never negative.
using System.Threading.Tasks;

var items = new System.Collections.Generic.List<long>();
long x = 12345;
for (long i = 0; i < 5000000; i++)
{
    x = (x * 1103515245 + 12345) % 2147483648;
    items.Add(x);
}
long[] sorted = SortOnCores(items.ToArray());
System.Console.WriteLine(sorted[0] + sorted[4999999]);

static long[] SortOnCores(long[] v)
{
    long all = 0;
    foreach (long y in v) all |= y;
    int shift = System.Math.Max(0, 64 - System.Numerics.BitOperations.LeadingZeroCount((ulong)all | 1) - 8);
    var start = new int[257];
    foreach (long y in v) start[(y >> shift) + 1]++;
    for (int b = 0; b < 256; b++) start[b + 1] += start[b];
    var output = new long[v.Length];
    var at = new int[256];
    System.Array.Copy(start, at, 256);
    foreach (long y in v) output[at[y >> shift]++] = y;
    Parallel.For(0, 256, b => System.Array.Sort(output, start[b], start[b + 1] - start[b]));
    return output;
}
