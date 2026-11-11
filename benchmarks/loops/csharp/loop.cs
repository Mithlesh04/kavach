// A tight `for` loop over integers: 500 million rounds.
long total = 0;
for (long i = 0; i < 500000000; i++)
{
    total = (total + i * 7 % 13) % 1000000007;
}
System.Console.WriteLine(total);
