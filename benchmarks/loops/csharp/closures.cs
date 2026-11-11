// Lambdas over a big list: map then filter then sum, 10 million items.
var items = new System.Collections.Generic.List<long>();
for (long i = 0; i < 10000000; i++)
{
    items.Add(i % 1000);
}
var doubled = new System.Collections.Generic.List<long>();
foreach (long x in items) doubled.Add(x * 2);
var big = new System.Collections.Generic.List<long>();
foreach (long x in doubled) if (x > 1000) big.Add(x);
long total = 0;
foreach (long x in big) total += x;
System.Console.WriteLine(total);
