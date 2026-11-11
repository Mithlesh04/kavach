// Nested lists: multiply two 300 x 300 matrices (27 million multiply-adds).
using System.Collections.Generic;

static List<List<double>> Make(long n, long seed)
{
    var rows = new List<List<double>>();
    for (long i = 0; i < n; i++)
    {
        var row = new List<double>();
        for (long j = 0; j < n; j++)
        {
            row.Add((double)((i * seed + j) % 10));
        }
        rows.Add(row);
    }
    return rows;
}

long n = 300;
var a = Make(n, 3);
var b = Make(n, 7);
double total = 0.0;
for (int i = 0; i < n; i++)
{
    var row = a[i];
    for (int j = 0; j < n; j++)
    {
        double cell = 0.0;
        for (int k = 0; k < n; k++)
        {
            cell += row[k] * b[k][j];
        }
        total += cell;
    }
}
System.Console.WriteLine(total.ToString("F1", System.Globalization.CultureInfo.InvariantCulture));
