// The maps benchmark when the program knows the size: 2 million keys
// into a map with room made for them, then look every key up.
var squares = new System.Collections.Generic.Dictionary<long, long>(2000000);
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
