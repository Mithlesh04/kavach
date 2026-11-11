// Sorting 5 million numbers.
var items = new System.Collections.Generic.List<long>();
long x = 12345;
for (long i = 0; i < 5000000; i++)
{
    x = (x * 1103515245 + 12345) % 2147483648;
    items.Add(x);
}
var sorted = new System.Collections.Generic.List<long>(items);
sorted.Sort();
System.Console.WriteLine(sorted[0] + sorted[4999999]);
