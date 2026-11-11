// A growing list: push 20 million numbers, then add them up.
var items = new System.Collections.Generic.List<long>();
for (long i = 0; i < 20000000; i++)
{
    items.Add(i % 1000);
}
long total = 0;
foreach (long x in items)
{
    total += x;
}
System.Console.WriteLine(total);
