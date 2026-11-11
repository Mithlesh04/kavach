// Recursive calls: fib(38) makes about 126 million calls.
static long Fib(long n) => n < 2 ? n : Fib(n - 1) + Fib(n - 2);

System.Console.WriteLine(Fib(38));
