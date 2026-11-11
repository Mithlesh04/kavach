// A map with 2 million keys: insert, then look every key up.
var squares = new System.Collections.Generic.Dictionary<long, long>();
for (long i = 0; i < 2000000; i++)
{
    squares[i] = i * i % 1000;
}
long total = 0;
for (long i = 0; i < 2000000; i++)
{
    total += squares.TryGetValue(i, out long v) ? v : 0;
}
System.Console.WriteLine(total);
