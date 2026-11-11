// Records: create 10 million points in a list, then read their fields.
var points = new System.Collections.Generic.List<Point>();
for (long i = 0; i < 10000000; i++)
{
    points.Add(new Point(i % 100, i % 7));
}
long total = 0;
foreach (var p in points)
{
    total += p.X * p.Y;
}
System.Console.WriteLine(total);

readonly record struct Point(long X, long Y);
